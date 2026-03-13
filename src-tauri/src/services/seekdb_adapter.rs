use anyhow::{anyhow, Result};
use async_trait::async_trait;
use seekdb_rs::{
    Client, DeleteQuery, DistanceMetric, EmbeddedDatabase, Filter, HnswConfig, HybridKnn,
    IncludeField, QueryParam, QueryResult, row_to_json_values, SeekDbError, UpsertBatch,
};
use seekdb_rs::EmbeddingFunction;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

const VECTOR_COLLECTION_NAME: &str = "vector_documents";
const VECTOR_DIMENSION: u32 = 1536;

fn embedding_f64_to_f32(v: &[f64]) -> Vec<f32> {
    v.iter().map(|&x| x as f32).collect()
}

/// 将文档/查询文本转为向量，委托 DashScope 服务，用于混合检索时对 query 文本做向量化。
pub struct DashScopeEmbeddingFunction {
    pub service: Arc<crate::services::dashscope_embedding_service::DashScopeEmbeddingService>,
}

#[async_trait]
impl EmbeddingFunction for DashScopeEmbeddingFunction {
    async fn embed_documents(&self, docs: &[String]) -> std::result::Result<Vec<Vec<f32>>, SeekDbError> {
        if docs.is_empty() {
            return Ok(Vec::new());
        }
        let texts: Vec<String> = docs.to_vec();
        let embeddings = self
            .service
            .embed_batch(&texts)
            .await
            .map_err(|e| SeekDbError::Embedding(e.to_string()))?;
        Ok(embeddings
            .into_iter()
            .map(|v: Vec<f64>| v.into_iter().map(|x| x as f32).collect::<Vec<f32>>())
            .collect::<Vec<Vec<f32>>>())
    }
    fn dimension(&self) -> usize {
        self.service.embedding_dim()
    }
}

fn doc_to_meta(doc: &VectorDocument) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("project_id".to_string(), json!(doc.project_id));
    m.insert("document_id".to_string(), json!(doc.document_id));
    m.insert("chunk_index".to_string(), json!(doc.chunk_index));
    for (k, v) in &doc.metadata {
        m.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(m)
}

fn meta_to_doc_meta(meta: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(obj) = meta.as_object() {
        for (k, v) in obj {
            if k == "project_id" || k == "document_id" || k == "chunk_index" {
                continue;
            }
            if let Some(s) = v.as_str() {
                out.insert(k.clone(), s.to_string());
            }
        }
    }
    out
}

fn query_result_to_search_results(qr: QueryResult, limit: usize) -> Vec<SearchResult> {
    let ids = qr.ids.get(0).map(|v| v.as_slice()).unwrap_or(&[]);
    let docs = qr.documents.as_ref().and_then(|d| d.get(0)).map(|v| v.as_slice()).unwrap_or(&[]);
    let metas = qr.metadatas.as_ref().and_then(|m| m.get(0)).map(|v| v.as_slice()).unwrap_or(&[]);
    let dists = qr.distances.as_ref().and_then(|d| d.get(0)).map(|v| v.as_slice()).unwrap_or(&[]);
    let mut results = Vec::new();
    for (i, id) in ids.iter().take(limit).enumerate() {
        let content = docs.get(i).cloned().unwrap_or_default();
        let meta = metas.get(i).cloned().unwrap_or(json!({}));
        let project_id = meta.get("project_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let document_id = meta.get("document_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let chunk_index = meta.get("chunk_index").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let distance = dists.get(i).copied().unwrap_or(0.0);
        let similarity = 1.0 / (1.0 + distance as f64);
        results.push(SearchResult {
            document: VectorDocument {
                id: id.clone(),
                project_id,
                document_id,
                chunk_index,
                content,
                embedding: vec![],
                metadata: meta_to_doc_meta(&meta),
            },
            similarity,
        });
    }
    results
}

/// Vector document structure (same as before)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub project_id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub embedding: Vec<f64>,
    pub metadata: HashMap<String, String>,
}

/// Search result structure (same as before)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: VectorDocument,
    pub similarity: f64,
}

#[derive(Clone)]
pub struct SeekDbAdapter {
    client: Client,
    hnsw_config: HnswConfig,
    db_path: String,
    db_name: String,
}

impl std::fmt::Debug for SeekDbAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeekDbAdapter")
            .field("db_path", &self.db_path)
            .field("db_name", &self.db_name)
            .finish_non_exhaustive()
    }
}

/// 将 JSON 值转为 SQL 参数（用于参数化查询）。
fn value_to_query_param(v: &Value) -> QueryParam {
    QueryParam::from_metadata_value(v)
}

fn seekdb_err(e: seekdb_rs::SeekDbError) -> anyhow::Error {
    anyhow!("SeekDB: {}", e)
}

/// 解析 DB 返回的 created_at，兼容两种格式，避免解析失败导致顺序错乱。
fn parse_datetime_from_db(s: &str) -> chrono::DateTime<chrono::Utc> {
    use chrono::{DateTime, NaiveDateTime, Utc};
    if s.is_empty() {
        return Utc::now();
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return DateTime::from_naive_utc_and_offset(naive, Utc);
    }
    Utc::now()
}

impl SeekDbAdapter {
    /// 使用 `db_path` 作为 SeekDB 实例目录；数据集中在该目录下。
    /// 异步构建，需在 async 上下文中调用。
    pub async fn new_async<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path_ref = db_path.as_ref();
        let db_dir = if db_path_ref.is_absolute() {
            db_path_ref.to_path_buf()
        } else {
            std::env::current_dir()?.join(db_path_ref)
        };
        let db_path_str = db_dir.display().to_string();
        let db_dir_str = db_path_str.clone();
        let db_name = db_dir
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.trim_end_matches(".db").replace('-', "_"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "mine_kb".to_string());

        log::info!("🔗 [NEW-DB] Opening embedded SeekDB: {}", db_path_str);
        log::info!("🔗 [NEW-DB] Database name: {}", db_name);

        let t_open = Instant::now();
        EmbeddedDatabase::open(&db_dir).map_err(|e| anyhow!("SeekDB open: {}", e))?;
        log::info!("🔗 [NEW-DB] Open data dir took {:?}", t_open.elapsed());

        let t_build = Instant::now();
        let client = Client::builder()
            .path(&db_dir_str)
            .database(&db_name)
            .build()
            .await
            .map_err(seekdb_err)?;
        log::info!("🔗 [NEW-DB] Build client took {:?}", t_build.elapsed());

        let hnsw_config = HnswConfig::new(VECTOR_DIMENSION, DistanceMetric::L2)
            .map_err(|e| anyhow!("HnswConfig: {}", e))?;

        let adapter = Self {
            client,
            hnsw_config,
            db_path: db_path_str,
            db_name,
        };
        let t_schema = Instant::now();
        adapter.initialize_schema().await?;
        log::info!("🔗 [NEW-DB] Init schema took {:?}", t_schema.elapsed());
        log::info!("🔗 [NEW-DB] Database ready");
        Ok(adapter)
    }

    async fn execute(&self, sql: &str, params: Vec<Value>) -> Result<()> {
        let params_q: Vec<QueryParam> = params.iter().map(value_to_query_param).collect();
        let params_ref = if params_q.is_empty() {
            None
        } else {
            Some(params_q.as_slice())
        };
        self.client.execute(sql, params_ref).await.map_err(seekdb_err)
    }

    async fn execute_no_params(&self, sql: &str) -> Result<()> {
        self.client.execute(sql, None).await.map_err(seekdb_err)
    }

    async fn query(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Vec<Value>>> {
        let params_q: Vec<QueryParam> = params.iter().map(value_to_query_param).collect();
        let params_ref = if params_q.is_empty() {
            None
        } else {
            Some(params_q.as_slice())
        };
        let max_cols = 64usize;
        let rows = self.client.fetch_all(sql, params_ref).await.map_err(seekdb_err)?;
        let converted: Vec<Vec<Value>> = rows
            .into_iter()
            .map(|r| row_to_json_values(r.as_ref(), max_cols))
            .collect();
        Ok(converted)
    }

    async fn query_one(&self, sql: &str, params: Vec<Value>) -> Result<Option<Vec<Value>>> {
        let rows = self.query(sql, params).await?;
        Ok(rows.into_iter().next())
    }

    async fn commit(&self) -> Result<()> {
        self.execute_no_params("COMMIT").await
    }

    /// Initialize database schema（每步打耗时日志，便于排查慢的根因）
    async fn initialize_schema(&self) -> Result<()> {
        log::info!("📋 Initializing schema...");

        let run = |name: String, sql: String| async move {
            let t = Instant::now();
            self.execute_no_params(&sql).await?;
            log::info!("📋 [schema] {} took {:?}", name, t.elapsed());
            Ok::<(), anyhow::Error>(())
        };

        run(
            "CREATE TABLE projects".to_string(),
            "CREATE TABLE IF NOT EXISTS projects (
                id VARCHAR(36) PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                document_count INTEGER DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )".to_string(),
        )
        .await?;

        run(
            "CREATE TABLE conversations".to_string(),
            "CREATE TABLE IF NOT EXISTS conversations (
                id VARCHAR(36) PRIMARY KEY,
                project_id VARCHAR(36) NOT NULL,
                title TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                message_count INTEGER DEFAULT 0,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                KEY idx_conversation_project_id(project_id)
            )".to_string(),
        )
        .await?;

        run(
            "CREATE TABLE messages".to_string(),
            "CREATE TABLE IF NOT EXISTS messages (
                id VARCHAR(36) PRIMARY KEY,
                conversation_id VARCHAR(36) NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                sources TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
                KEY idx_message_conversation_id(conversation_id)
            )".to_string(),
        )
        .await?;

        let t_commit = Instant::now();
        self.commit().await?;
        log::info!("📋 [schema] COMMIT took {:?}", t_commit.elapsed());
        log::info!("✅ Database schema initialized");
        Ok(())
    }

    /// Add a single vector document.
    pub async fn add_document(&self, doc: VectorDocument) -> Result<()> {
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let id = doc.id.clone();
        let emb = embedding_f64_to_f32(&doc.embedding);
        let meta = doc_to_meta(&doc);
        let content = doc.content.clone();
        coll.upsert_batch(
            UpsertBatch::new(&[id])
                .embeddings(&[emb])
                .metadatas(&[meta])
                .documents(&[content]),
        )
        .await
        .map_err(seekdb_err)
    }

    /// Add multiple vector documents (batch upsert).
    pub async fn add_documents(&self, docs: Vec<VectorDocument>) -> Result<()> {
        if docs.is_empty() {
            return Ok(());
        }
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let ids: Vec<String> = docs.iter().map(|d| d.id.clone()).collect();
        let embeddings: Vec<Vec<f32>> =
            docs.iter().map(|d| embedding_f64_to_f32(&d.embedding)).collect();
        let metadatas: Vec<Value> = docs.iter().map(doc_to_meta).collect();
        let contents: Vec<String> = docs.iter().map(|d| d.content.clone()).collect();
        coll.upsert_batch(
            UpsertBatch::new(&ids)
                .embeddings(&embeddings)
                .metadatas(&metadatas)
                .documents(&contents),
        )
        .await
        .map_err(seekdb_err)
    }

    /// 向量 KNN 检索，可按 project_id 过滤。
    pub async fn hybrid_search(
        &self,
        _query_text: &str,
        query_embedding: &[f64],
        project_id: Option<&str>,
        limit: usize,
        _semantic_boost: f64,
    ) -> Result<Vec<SearchResult>> {
        log::info!("🔍 [HYBRID-SEARCH] 向量 KNN 检索");
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let query_emb = embedding_f64_to_f32(query_embedding);
        let where_meta = project_id
            .map(|pid| Filter::Eq { field: "project_id".to_string(), value: json!(pid) });
        let limit_u = limit as u32;
        let qr = coll
            .query_embeddings(
                &[query_emb],
                limit_u,
                where_meta.as_ref(),
                None,
                Some(&[IncludeField::Documents, IncludeField::Metadatas]),
            )
            .await
            .map_err(seekdb_err)?;
        let results = query_result_to_search_results(qr, limit);
        log::info!("✅ [HYBRID-SEARCH] 返回 {} 个结果", results.len());
        Ok(results)
    }

    /// 混合检索（关键词+向量）：用 query 文本直接检索，内部对 query 做向量化并执行混合搜索。
    pub async fn hybrid_search_by_text(
        &self,
        embedding_service: Arc<crate::services::dashscope_embedding_service::DashScopeEmbeddingService>,
        project_id: Option<&str>,
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        log::info!("🔍 [HYBRID-BY-TEXT] 混合检索（关键词+向量）");
        let ef = DashScopeEmbeddingFunction { service: embedding_service };
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                Some(ef),
            )
            .await
            .map_err(seekdb_err)?;
        let query_text = query_text.to_string();
        let where_meta = project_id
            .map(|pid| Filter::Eq { field: "project_id".to_string(), value: json!(pid) });
        let limit_u = limit as u32;
        let knn = HybridKnn {
            query_texts: Some(vec![query_text]),
            query_embeddings: None,
            where_meta,
            n_results: Some(limit_u),
        };
        let qr = coll
            .hybrid_search_advanced(
                None,
                Some(knn),
                None,
                limit_u,
                Some(&[IncludeField::Documents, IncludeField::Metadatas]),
            )
            .await
            .map_err(seekdb_err)?;
        let results = query_result_to_search_results(qr, limit);
        log::info!("✅ [HYBRID-BY-TEXT] 返回 {} 个结果", results.len());
        Ok(results)
    }

    /// 向量相似度检索（L2 距离），按 threshold 过滤后截断条数。
    pub async fn similarity_search(
        &self,
        query_embedding: &[f64],
        project_id: Option<&str>,
        limit: usize,
        threshold: f64,
    ) -> Result<Vec<SearchResult>> {
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let query_emb = embedding_f64_to_f32(query_embedding);
        let where_meta = project_id
            .map(|pid| Filter::Eq { field: "project_id".to_string(), value: json!(pid) });
        let limit_u = (limit * 2).min(1000) as u32;
        let qr = coll
            .query_embeddings(
                &[query_emb],
                limit_u,
                where_meta.as_ref(),
                None,
                Some(&[IncludeField::Documents, IncludeField::Metadatas]),
            )
            .await
            .map_err(seekdb_err)?;
        let mut results = query_result_to_search_results(qr, limit_u as usize);
        results.retain(|r| r.similarity >= threshold);
        results.truncate(limit);
        Ok(results)
    }

    /// 获取项目下所有向量文档（按 project_id 过滤）。
    pub async fn get_project_documents(&self, project_id: &str) -> Result<Vec<VectorDocument>> {
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let filter =
            Filter::Eq { field: "project_id".to_string(), value: json!(project_id) };
        let get_result = coll
            .get(
                None,
                Some(&filter),
                None,
                Some(100_000),
                Some(0),
                Some(&[IncludeField::Documents, IncludeField::Metadatas]),
            )
            .await
            .map_err(seekdb_err)?;
        let ids = get_result.ids;
        let docs = get_result.documents.unwrap_or_default();
        let metas = get_result.metadatas.unwrap_or_default();
        let mut documents = Vec::new();
        for (i, id) in ids.into_iter().enumerate() {
            let content = docs.get(i).cloned().unwrap_or_default();
            let meta = metas.get(i).cloned().unwrap_or(json!({}));
            let project_id_str =
                meta.get("project_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let document_id =
                meta.get("document_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let chunk_index =
                meta.get("chunk_index").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            documents.push(VectorDocument {
                id,
                project_id: project_id_str,
                document_id,
                chunk_index,
                content,
                embedding: vec![],
                metadata: meta_to_doc_meta(&meta),
            });
        }
        documents.sort_by(|a, b| {
            match a.document_id.cmp(&b.document_id) {
                std::cmp::Ordering::Equal => a.chunk_index.cmp(&b.chunk_index),
                other => other,
            }
        });
        Ok(documents)
    }

    /// 按项目删除向量文档。
    pub async fn delete_project_documents(&self, project_id: &str) -> Result<usize> {
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let filter =
            Filter::Eq { field: "project_id".to_string(), value: json!(project_id) };
        coll.delete_query(DeleteQuery::new().with_where_meta(&filter))
            .await
            .map_err(seekdb_err)?;
        Ok(0)
    }

    /// 按 document_id 删除向量文档。
    pub async fn delete_document(&self, document_id: &str) -> Result<usize> {
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let filter =
            Filter::Eq { field: "document_id".to_string(), value: json!(document_id) };
        coll.delete_query(DeleteQuery::new().with_where_meta(&filter))
            .await
            .map_err(seekdb_err)?;
        Ok(0)
    }

    /// 从查询结果解析整型（兼容 Number 或 String 列）。
    fn value_as_i64(v: &Value) -> i64 {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            .unwrap_or(0)
    }

    /// 从查询结果解析浮点（兼容 Number 或 String 列，如 distance）。
    fn value_as_f64(v: &Value) -> f64 {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
            .unwrap_or(f64::MAX)
    }

    /// 统计：向量总条数 + 项目数（项目数来自 projects 表）。
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>> {
        let mut stats = HashMap::new();
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let total_documents = coll.count().await.map_err(seekdb_err)?;
        stats.insert("total_documents".to_string(), total_documents as i64);
        if let Some(row) = self.query_one("SELECT COUNT(*) FROM projects", vec![]).await? {
            stats.insert("total_projects".to_string(), Self::value_as_i64(&row[0]));
        }
        Ok(stats)
    }

    /// 统计指定项目下的向量条数。
    pub async fn count_project_documents(&self, project_id: &str) -> Result<usize> {
        let coll = self
            .client
            .get_or_create_collection::<DashScopeEmbeddingFunction>(
                VECTOR_COLLECTION_NAME,
                Some(self.hnsw_config.clone()),
                None,
            )
            .await
            .map_err(seekdb_err)?;
        let filter =
            Filter::Eq { field: "project_id".to_string(), value: json!(project_id) };
        let res = coll
            .get(None, Some(&filter), None, Some(100_000), Some(0), None)
            .await
            .map_err(seekdb_err)?;
        Ok(res.ids.len())
    }

    /// Save project to database
    pub async fn save_project(&self, project: &crate::models::project::Project) -> Result<()> {
        log::info!("💾 [SAVE-PROJECT] Saving project: id={}, name={}", project.id, project.name);

        self.execute(
            "INSERT INTO projects (id, name, description, status, document_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE
                name = VALUES(name),
                description = VALUES(description),
                status = VALUES(status),
                document_count = VALUES(document_count),
                updated_at = VALUES(updated_at)",
            vec![
                Value::String(project.id.to_string()),
                Value::String(project.name.clone()),
                Value::String(project.description.clone().unwrap_or_default()),
                Value::String(project.status.to_string()),
                Value::Number((project.document_count as i64).into()),
                Value::String(project.created_at.to_rfc3339()),
                Value::String(project.updated_at.to_rfc3339()),
            ],
        )
        .await?;
        self.commit().await?;
        log::info!("💾 [SAVE-PROJECT] Project saved successfully");
        Ok(())
    }

    /// Load all projects from database
    pub async fn load_all_projects(&self) -> Result<Vec<crate::models::project::Project>> {
        use chrono::DateTime;
        use uuid::Uuid;

        let rows = self.query(
            "SELECT id, name, description, status, document_count, created_at, updated_at
             FROM projects",
            vec![],
        )
        .await?;

        let mut projects = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if row.len() < 7 {
                log::warn!("跳过项目 #{}: 列数不足 ({})", idx, row.len());
                continue;
            }
            let id_str = row[0].as_str().unwrap_or_default();
            if id_str.is_empty() {
                continue;
            }
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let name = row[1].as_str().unwrap_or_default().to_string();
            let description = row[2].as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
            let status_str = row[3].as_str().unwrap_or("Created");
            let status = match status_str {
                "Processing" => crate::models::project::ProjectStatus::Processing,
                "Ready" => crate::models::project::ProjectStatus::Ready,
                "Error" => crate::models::project::ProjectStatus::Error,
                _ => crate::models::project::ProjectStatus::Created,
            };
            let document_count = Self::value_as_i64(&row[4]) as u32;
            let created_at_str = row[5].as_str().unwrap_or_default();
            let created_at = if created_at_str.is_empty() {
                chrono::Utc::now()
            } else {
                DateTime::parse_from_rfc3339(created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now())
            };
            let updated_at_str = row[6].as_str().unwrap_or_default();
            let updated_at = if updated_at_str.is_empty() {
                created_at
            } else {
                DateTime::parse_from_rfc3339(updated_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(created_at)
            };

            projects.push(crate::models::project::Project {
                id,
                name,
                description,
                status,
                document_count,
                created_at,
                updated_at,
            });
        }
        projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(projects)
    }

    /// Delete project by ID
    pub async fn delete_project_by_id(&self, project_id: &str) -> Result<usize> {
        self.execute("DELETE FROM projects WHERE id = ?", vec![Value::String(project_id.to_string())])
            .await?;
        self.commit().await?;
        Ok(0)
    }

    /// Update project document count
    pub async fn update_project_document_count(&self, project_id: &str, count: u32) -> Result<()> {
        self.execute(
            "UPDATE projects SET document_count = ?, updated_at = NOW() WHERE id = ?",
            vec![
                Value::Number((count as i64).into()),
                Value::String(project_id.to_string()),
            ],
        )
        .await?;
        self.commit().await?;
        Ok(())
    }

    /// Save conversation to database
    pub async fn save_conversation(
        &self,
        conversation: &crate::models::conversation::Conversation,
    ) -> Result<()> {
        log::info!("💾 [SAVE-CONV] Saving conversation: id={}", conversation.id);

        self.execute(
            "INSERT INTO conversations (id, project_id, title, created_at, updated_at, message_count)
             VALUES (?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE
                title = VALUES(title),
                updated_at = VALUES(updated_at),
                message_count = VALUES(message_count)",
            vec![
                Value::String(conversation.id.to_string()),
                Value::String(conversation.project_id.to_string()),
                Value::String(conversation.title.clone()),
                Value::String(conversation.created_at.to_rfc3339()),
                Value::String(conversation.updated_at.to_rfc3339()),
                Value::Number((conversation.message_count as i64).into()),
            ],
        )
        .await?;
        self.commit().await?;
        log::info!("💾 [SAVE-CONV] Conversation saved successfully");
        Ok(())
    }

    /// Load conversations by project
    pub async fn load_conversations_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<crate::models::conversation::Conversation>> {
        use chrono::DateTime;
        use uuid::Uuid;

        let rows = self.query(
            "SELECT id, project_id, title, created_at, updated_at, message_count
             FROM conversations
             WHERE project_id = ?",
            vec![Value::String(project_id.to_string())],
        )
        .await?;

        let mut conversations = Vec::new();
        for row in rows.iter() {
            if row.len() < 6 {
                continue;
            }
            let id_str = row[0].as_str().unwrap_or_default();
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let project_id_str = row[1].as_str().unwrap_or_default();
            let project_id = match Uuid::parse_str(project_id_str) {
                Ok(pid) => pid,
                Err(_) => continue,
            };
            let title = row[2].as_str().unwrap_or_default().to_string();
            let created_at_str = row[3].as_str().unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at_str = row[4].as_str().unwrap_or_default();
            let updated_at = DateTime::parse_from_rfc3339(updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or(created_at);
            let message_count = Self::value_as_i64(&row[5]) as u32;

            conversations.push(crate::models::conversation::Conversation {
                id,
                project_id,
                title,
                created_at,
                updated_at,
                message_count,
            });
        }
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(conversations)
    }

    /// Load all conversations
    pub async fn load_all_conversations(&self) -> Result<Vec<crate::models::conversation::Conversation>> {
        use chrono::DateTime;
        use uuid::Uuid;

        let rows = self.query(
            "SELECT id, project_id, title, created_at, updated_at, message_count
             FROM conversations",
            vec![],
        )
        .await?;

        let mut conversations = Vec::new();
        for row in rows.iter() {
            if row.len() < 6 {
                continue;
            }
            let id_str = row[0].as_str().unwrap_or_default();
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let project_id_str = row[1].as_str().unwrap_or_default();
            let project_id = match Uuid::parse_str(project_id_str) {
                Ok(pid) => pid,
                Err(_) => continue,
            };
            let title = row[2].as_str().unwrap_or_default().to_string();
            let created_at_str = row[3].as_str().unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at_str = row[4].as_str().unwrap_or_default();
            let updated_at = DateTime::parse_from_rfc3339(updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or(created_at);
            let message_count = Self::value_as_i64(&row[5]) as u32;

            conversations.push(crate::models::conversation::Conversation {
                id,
                project_id,
                title,
                created_at,
                updated_at,
                message_count,
            });
        }
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(conversations)
    }

    /// Delete conversation by ID
    pub async fn delete_conversation_by_id(&self, conversation_id: &str) -> Result<usize> {
        self.execute(
            "DELETE FROM conversations WHERE id = ?",
            vec![Value::String(conversation_id.to_string())],
        )
        .await?;
        self.commit().await?;
        Ok(0)
    }

    /// Delete message by ID
    pub async fn delete_message_by_id(&self, message_id: &str) -> Result<usize> {
        self.execute(
            "DELETE FROM messages WHERE id = ?",
            vec![Value::String(message_id.to_string())],
        )
        .await?;
        self.commit().await?;
        Ok(0)
    }

    /// Delete all messages in a conversation
    pub async fn delete_messages_by_conversation(&self, conversation_id: &str) -> Result<usize> {
        self.execute(
            "DELETE FROM messages WHERE conversation_id = ?",
            vec![Value::String(conversation_id.to_string())],
        )
        .await?;
        self.commit().await?;
        Ok(0)
    }

    /// Save message to database
    pub async fn save_message(&self, message: &crate::models::conversation::Message) -> Result<()> {
        log::info!("📝 [SAVE-MSG] Saving message: id={}", message.id);

        let sources_json = message
            .sources
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok());

        let insert_result = self.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at, sources)
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(message.id.to_string()),
                Value::String(message.conversation_id.to_string()),
                Value::String(message.role.to_string()),
                Value::String(message.content.clone()),
                Value::String(message.timestamp.to_rfc3339()),
                sources_json
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            ],
        )
        .await;

        match insert_result {
            Ok(()) => {}
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Duplicated primary key") || error_msg.contains("1062") {
                    self.execute(
                        "UPDATE messages SET role=?, content=?, created_at=?, sources=? WHERE id=?",
                        vec![
                            Value::String(message.role.to_string()),
                            Value::String(message.content.clone()),
                            Value::String(message.timestamp.to_rfc3339()),
                            sources_json
                                .map(Value::String)
                                .unwrap_or(Value::Null),
                            Value::String(message.id.to_string()),
                        ],
                    )
                    .await?;
                } else {
                    return Err(e);
                }
            }
        }
        self.commit().await?;
        log::info!("📝 [SAVE-MSG] Message saved successfully");
        Ok(())
    }

    /// Get message count
    pub async fn get_message_count(&self) -> Result<i32> {
        if let Some(row) = self.query_one("SELECT COUNT(*) FROM messages", vec![]).await? {
            return Ok(Self::value_as_i64(&row[0]) as i32);
        }
        Ok(0)
    }

    /// Get conversation message count
    pub async fn get_conversation_message_count(&self, conversation_id: &str) -> Result<i32> {
        if let Some(row) = self.query_one(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ?",
            vec![Value::String(conversation_id.to_string())],
        )
        .await?
        {
            return Ok(Self::value_as_i64(&row[0]) as i32);
        }
        Ok(0)
    }

    /// Load messages by conversation
    pub async fn load_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<crate::models::conversation::Message>> {
        use uuid::Uuid;

        let rows = self.query(
            "SELECT id, conversation_id, role, content, created_at, sources
             FROM messages
             WHERE conversation_id = ?",
            vec![Value::String(conversation_id.to_string())],
        )
        .await?;

        let mut messages = Vec::new();
        for row in rows.iter() {
            if row.len() < 6 {
                continue;
            }
            let id_str = row[0].as_str().unwrap_or_default();
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let conversation_id_str = row[1].as_str().unwrap_or_default();
            let conversation_id = match Uuid::parse_str(conversation_id_str) {
                Ok(cid) => cid,
                Err(_) => continue,
            };
            let role_str = row[2].as_str().unwrap_or("User");
            let role = match role_str {
                "Assistant" | "assistant" => crate::models::conversation::MessageRole::Assistant,
                "System" | "system" => crate::models::conversation::MessageRole::System,
                _ => crate::models::conversation::MessageRole::User,
            };
            let content = row[3].as_str().unwrap_or_default().to_string();
            let created_at_str = row[4].as_str().unwrap_or_default();
            let created_at = parse_datetime_from_db(created_at_str);
            let sources = row[5]
                .as_str()
                .and_then(|s| if s.is_empty() { None } else { serde_json::from_str(s).ok() });

            messages.push(crate::models::conversation::Message {
                id,
                conversation_id,
                role,
                content,
                timestamp: created_at,
                token_count: 0,
                context_chunks: Vec::new(),
                processing_time: None,
                sources,
            });
        }
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then_with(|| a.id.cmp(&b.id)));
        Ok(messages)
    }

    /// Verify database connection
    pub async fn verify_connection(&self) -> Result<()> {
        log::info!("🔍 验证 SeekDB 数据库连接...");
        match self.query("SELECT 1", vec![]).await {
            Ok(rows) => {
                if rows.is_empty() || rows[0].is_empty() {
                    return Err(anyhow!("数据库查询返回空结果"));
                }
                log::info!("✅ SeekDB 数据库连接正常");
                Ok(())
            }
            Err(e) => {
                log::error!("❌ SeekDB 数据库连接验证失败: {}", e);
                Err(anyhow!("数据库连接验证失败: {}", e))
            }
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        log::info!("🏥 执行 SeekDB 健康检查...");
        self.verify_connection().await?;
        log::info!("✅ SeekDB 健康检查通过");
        Ok(())
    }
}
