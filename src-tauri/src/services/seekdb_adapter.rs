use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::python_subprocess::PythonSubprocess;

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

/// SeekDB adapter - manages database operations through Python subprocess
#[derive(Clone, Debug)]
pub struct SeekDbAdapter {
    subprocess: Arc<Mutex<PythonSubprocess>>,
    db_path: String,
    db_name: String,
}

impl SeekDbAdapter {
    /// Create new SeekDB adapter instance
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Self::new_with_python(db_path, "python3")
    }
    
    /// Create new SeekDB adapter instance with custom Python executable
    pub fn new_with_python<P: AsRef<Path>>(db_path: P, python_executable: &str) -> Result<Self> {
        let db_path_str = db_path.as_ref().display().to_string();
        log::info!("🔗 [NEW-DB] Opening SeekDB: {}", db_path_str);
        
        // Get absolute path for database directory
        let db_dir = if db_path.as_ref().is_absolute() {
            db_path.as_ref().parent().unwrap().to_path_buf()
        } else {
            std::env::current_dir()?.join(db_path.as_ref()).parent().unwrap().to_path_buf()
        };
        
        // Get the database file name (without extension) and normalize it
        // Replace hyphens with underscores for SQL compatibility
        let db_name = db_path.as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("mine_kb")
            .replace("-", "_");  // Normalize: mine-kb -> mine_kb
        
        log::info!("🔗 [NEW-DB] Database directory: {:?}", db_dir);
        log::info!("🔗 [NEW-DB] Database name: {}", db_name);
        log::info!("🔗 [NEW-DB] Python executable: {}", python_executable);
        
        // Determine Python script path with multiple fallbacks
        let script_path = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("python/seekdb_bridge.py")))
            .filter(|p| p.exists())
            .or_else(|| {
                // Try to find script relative to current directory
                if let Ok(cwd) = std::env::current_dir() {
                    log::debug!("🔍 Current directory: {:?}", cwd);
                    
                    // Try multiple possible locations
                    let candidates = vec![
                        cwd.join("python/seekdb_bridge.py"),                // If in src-tauri
                        cwd.join("src-tauri/python/seekdb_bridge.py"),      // If in project root
                        cwd.parent()?.join("python/seekdb_bridge.py"),      // If in src-tauri/src
                    ];
                    
                    for candidate in candidates {
                        log::debug!("🔍 Checking: {:?}", candidate);
                        if candidate.exists() {
                            log::info!("✅ Found script at: {:?}", candidate);
                            return Some(candidate);
                        }
                    }
                }
                None
            })
            .unwrap_or_else(|| {
                // Last resort: use relative path and hope for the best
                log::warn!("⚠️ Could not find seekdb_bridge.py in expected locations");
                std::path::PathBuf::from("src-tauri/python/seekdb_bridge.py")
            });
        
        log::info!("🔗 [NEW-DB] Python script: {:?}", script_path);
        
        // Start Python subprocess with specified Python executable
        let subprocess = PythonSubprocess::new_with_python(
            script_path.to_str().unwrap(),
            python_executable
        )?;
        
        // Initialize database - use the actual db_path passed to the function
        subprocess.init_db(&db_path_str, &db_name)?;
        
        let adapter = Self {
            subprocess: Arc::new(Mutex::new(subprocess)),
            db_path: db_path_str.clone(),
            db_name: db_name.clone(),
        };
        
        // Initialize schema
        adapter.initialize_schema()?;
        
        log::info!("🔗 [NEW-DB] SeekDB adapter initialized successfully");
        
        Ok(adapter)
    }
    
    /// Initialize database schema
    fn initialize_schema(&self) -> Result<()> {
        log::info!("📋 Initializing database schema...");
        
        let subprocess = self.subprocess.lock().unwrap();
        
        // Create projects table
        subprocess.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id VARCHAR(36) PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                document_count INTEGER DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )",
            vec![],
        )?;
        
        // Create vector_documents table with vector index and fulltext index for hybrid search
        subprocess.execute(
            "CREATE TABLE IF NOT EXISTS vector_documents (
                id VARCHAR(36) PRIMARY KEY,
                project_id VARCHAR(36) NOT NULL,
                document_id VARCHAR(36) NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                embedding vector(1536),
                metadata TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(document_id, chunk_index),
                VECTOR INDEX idx_embedding(embedding) WITH (distance=l2, type=hnsw, lib=vsag),
                FULLTEXT idx_content(content)
            )",
            vec![],
        )?;
        
        // Create regular indexes
        subprocess.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_id ON vector_documents(project_id)",
            vec![],
        )?;
        
        subprocess.execute(
            "CREATE INDEX IF NOT EXISTS idx_document_id ON vector_documents(document_id)",
            vec![],
        )?;
        
        // Create conversations table
        subprocess.execute(
            "CREATE TABLE IF NOT EXISTS conversations (
                id VARCHAR(36) PRIMARY KEY,
                project_id VARCHAR(36) NOT NULL,
                title TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                message_count INTEGER DEFAULT 0,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            )",
            vec![],
        )?;
        
        // Create messages table
        subprocess.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id VARCHAR(36) PRIMARY KEY,
                conversation_id VARCHAR(36) NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                sources TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
            vec![],
        )?;
        
        // Create conversation indexes
        subprocess.execute(
            "CREATE INDEX IF NOT EXISTS idx_conversation_project_id ON conversations(project_id)",
            vec![],
        )?;
        
        subprocess.execute(
            "CREATE INDEX IF NOT EXISTS idx_message_conversation_id ON messages(conversation_id)",
            vec![],
        )?;
        
        // Commit schema changes
        subprocess.commit()?;
        
        log::info!("✅ Database schema initialized");
        Ok(())
    }
    
    /// Add a single vector document
    pub fn add_document(&mut self, doc: VectorDocument) -> Result<()> {
        let subprocess = self.subprocess.lock().unwrap();
        
        let metadata_json = serde_json::to_string(&doc.metadata)?;
        
        // Convert embedding to JSON array string format for SeekDB
        let embedding_str = format!("[{}]", 
            doc.embedding.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        subprocess.execute(
            "INSERT INTO vector_documents 
             (id, project_id, document_id, chunk_index, content, embedding, metadata, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, NOW())
             ON DUPLICATE KEY UPDATE 
                content = VALUES(content),
                embedding = VALUES(embedding),
                metadata = VALUES(metadata)",
            vec![
                Value::String(doc.id),
                Value::String(doc.project_id),
                Value::String(doc.document_id),
                Value::Number(doc.chunk_index.into()),
                Value::String(doc.content),
                Value::String(embedding_str),
                Value::String(metadata_json),
            ],
        )?;
        
        Ok(())
    }
    
    /// Add multiple vector documents in a transaction
    pub fn add_documents(&mut self, docs: Vec<VectorDocument>) -> Result<()> {
        let subprocess = self.subprocess.lock().unwrap();
        
        for doc in docs {
            let metadata_json = serde_json::to_string(&doc.metadata)?;
            let embedding_str = format!("[{}]", 
                doc.embedding.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            
            subprocess.execute(
                "INSERT INTO vector_documents 
                 (id, project_id, document_id, chunk_index, content, embedding, metadata, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, NOW())
                 ON DUPLICATE KEY UPDATE 
                    content = VALUES(content),
                    embedding = VALUES(embedding),
                    metadata = VALUES(metadata)",
                vec![
                    Value::String(doc.id),
                    Value::String(doc.project_id),
                    Value::String(doc.document_id),
                    Value::Number(doc.chunk_index.into()),
                    Value::String(doc.content),
                    Value::String(embedding_str),
                    Value::String(metadata_json),
                ],
            )?;
        }
        
        subprocess.commit()?;
        Ok(())
    }
    
    /// Hybrid search using SeekDB's native hybrid search (vector + fulltext)
    pub fn hybrid_search(
        &self,
        query_text: &str,
        query_embedding: &[f64],
        project_id: Option<&str>,
        limit: usize,
        semantic_boost: f64,
    ) -> Result<Vec<SearchResult>> {
        log::info!("🔍 [HYBRID-SEARCH] 开始混合检索");
        log::info!("   查询文本: {}", query_text);
        log::info!("   向量维度: {}", query_embedding.len());
        log::info!("   项目ID: {:?}", project_id);
        log::info!("   返回数量: {}", limit);
        log::info!("   语义权重: {}", semantic_boost);
        
        let subprocess = self.subprocess.lock().unwrap();
        
        // Convert query embedding to JSON array
        let embedding_json = format!("[{}]", 
            query_embedding.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        // Build hybrid search query using dbms_hybrid_search.search()
        // Reference: docs/seekdb.md section 3.3
        let search_param = if let Some(pid) = project_id {
            format!(r#"{{
                "query": {{
                    "bool": {{
                        "must": [
                            {{"match": {{"content": "{}"}}}}
                        ]
                    }}
                }},
                "knn": {{
                    "field": "embedding",
                    "k": {},
                    "num_candidates": {},
                    "query_vector": {},
                    "boost": {}
                }},
                "filter": {{
                    "term": {{"project_id": "{}"}}
                }},
                "_source": ["id", "project_id", "document_id", "chunk_index", "content", "metadata", "_keyword_score", "_semantic_score"]
            }}"#, 
                query_text.replace('"', "\\\""),
                limit,
                limit * 2,
                embedding_json,
                semantic_boost,
                pid
            )
        } else {
            format!(r#"{{
                "query": {{
                    "bool": {{
                        "must": [
                            {{"match": {{"content": "{}"}}}}
                        ]
                    }}
                }},
                "knn": {{
                    "field": "embedding",
                    "k": {},
                    "num_candidates": {},
                    "query_vector": {},
                    "boost": {}
                }},
                "_source": ["id", "project_id", "document_id", "chunk_index", "content", "metadata", "_keyword_score", "_semantic_score"]
            }}"#,
                query_text.replace('"', "\\\""),
                limit,
                limit * 2,
                embedding_json,
                semantic_boost
            )
        };
        
        log::debug!("混合搜索参数: {}", search_param);
        
        // Set the parameter variable
        subprocess.execute(
            &format!("SET @search_param = '{}'", search_param.replace('\'', "\\'")),
            vec![],
        )?;
        
        // Execute hybrid search
        let rows = subprocess.query(
            "SELECT dbms_hybrid_search.search('vector_documents', @search_param)",
            vec![],
        )?;
        
        log::info!("✅ [HYBRID-SEARCH] 混合检索返回 {} 行结果", rows.len());
        
        // Parse results
        let mut results = Vec::new();
        for row in rows {
            if row.is_empty() {
                continue;
            }
            
            // The result is a JSON string
            let result_json = row[0].as_str().unwrap_or("{}");
            log::debug!("结果 JSON: {}", result_json);
            
            // Parse the JSON result
            if let Ok(result_obj) = serde_json::from_str::<serde_json::Value>(result_json) {
                if let Some(hits) = result_obj["hits"]["hits"].as_array() {
                    for hit in hits {
                        let source = &hit["_source"];
                        let id = source["id"].as_str().unwrap_or_default().to_string();
                        let project_id = source["project_id"].as_str().unwrap_or_default().to_string();
                        let document_id = source["document_id"].as_str().unwrap_or_default().to_string();
                        let chunk_index = source["chunk_index"].as_i64().unwrap_or(0) as i32;
                        let content = source["content"].as_str().unwrap_or_default().to_string();
                        
                        // Get scores
                        let keyword_score = source["_keyword_score"].as_f64().unwrap_or(0.0);
                        let semantic_score = source["_semantic_score"].as_f64().unwrap_or(0.0);
                        let total_score = hit["_score"].as_f64().unwrap_or(0.0);
                        
                        log::debug!("  文档ID: {}, 关键词分数: {:.4}, 语义分数: {:.4}, 总分: {:.4}",
                            document_id, keyword_score, semantic_score, total_score);
                        
                        // Parse metadata
                        let metadata_str = source["metadata"].as_str().unwrap_or("{}");
                        let metadata: HashMap<String, String> = serde_json::from_str(metadata_str).unwrap_or_default();
                        
                        // We don't have the embedding in the result, use empty vector
                        results.push(SearchResult {
                            document: VectorDocument {
                                id,
                                project_id,
                                document_id,
                                chunk_index,
                                content,
                                embedding: vec![],
                                metadata,
                            },
                            similarity: total_score,
                        });
                    }
                }
            }
        }
        
        log::info!("✅ [HYBRID-SEARCH] 解析得到 {} 个有效结果", results.len());
        
        Ok(results)
    }
    
    /// Vector similarity search using SeekDB's native L2 distance
    pub fn similarity_search(
        &self,
        query_embedding: &[f64],
        project_id: Option<&str>,
        limit: usize,
        threshold: f64,
    ) -> Result<Vec<SearchResult>> {
        let subprocess = self.subprocess.lock().unwrap();
        
        // Convert query embedding to SeekDB format
        let embedding_str = format!("[{}]", 
            query_embedding.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        // Build SQL query with SeekDB's native vector search
        // Note: We don't SELECT the embedding field because SeekDB doesn't support
        // fetching vector columns when using vector functions (l2_distance) with APPROXIMATE
        let sql = if project_id.is_some() {
            format!(
                "SELECT id, project_id, document_id, chunk_index, content, metadata,
                        l2_distance(embedding, '{}') as distance
                 FROM vector_documents
                 WHERE project_id = ?
                 ORDER BY l2_distance(embedding, '{}') APPROXIMATE
                 LIMIT {}",
                embedding_str, embedding_str, limit * 2 // Get more to filter by threshold
            )
        } else {
            format!(
                "SELECT id, project_id, document_id, chunk_index, content, metadata,
                        l2_distance(embedding, '{}') as distance
                 FROM vector_documents
                 ORDER BY l2_distance(embedding, '{}') APPROXIMATE
                 LIMIT {}",
                embedding_str, embedding_str, limit * 2
            )
        };
        
        let values = if project_id.is_some() {
            vec![Value::String(project_id.unwrap().to_string())]
        } else {
            vec![]
        };
        
        let rows = subprocess.query(&sql, values)?;
        
        let mut results = Vec::new();
        for row in rows {
            if row.len() < 7 {
                continue;
            }
            
            let id = row[0].as_str().unwrap_or_default().to_string();
            let project_id = row[1].as_str().unwrap_or_default().to_string();
            let document_id = row[2].as_str().unwrap_or_default().to_string();
            let chunk_index = row[3].as_i64().unwrap_or(0) as i32;
            let content = row[4].as_str().unwrap_or_default().to_string();
            
            // Parse metadata
            let metadata_str = row[5].as_str().unwrap_or("{}");
            let metadata: HashMap<String, String> = serde_json::from_str(metadata_str).unwrap_or_default();
            
            // Get distance (L2) and convert to similarity (inverse)
            let distance = row[6].as_f64().unwrap_or(f64::MAX);
            
            // Convert L2 distance to cosine similarity approximation
            // For normalized vectors, cosine similarity ≈ 1 - (L2_distance^2 / 2)
            // But since we don't know if vectors are normalized, we'll use a simple inverse
            let similarity = if distance > 0.0 {
                1.0 / (1.0 + distance)
            } else {
                1.0
            };
            
            // Filter by threshold
            if similarity >= threshold {
                results.push(SearchResult {
                    document: VectorDocument {
                        id,
                        project_id,
                        document_id,
                        chunk_index,
                        content,
                        embedding: vec![], // Empty vector - not returned by query for performance
                        metadata,
                    },
                    similarity,
                });
            }
        }
        
        // Limit results
        results.truncate(limit);
        
        Ok(results)
    }
    
    /// Get all documents for a project
    pub fn get_project_documents(&self, project_id: &str) -> Result<Vec<VectorDocument>> {
        let subprocess = self.subprocess.lock().unwrap();
        
        // Note: SeekDB may not support selecting vector columns in all contexts
        // We query without embedding field and use empty vectors
        let rows = subprocess.query(
            "SELECT id, project_id, document_id, chunk_index, content, metadata
             FROM vector_documents
             WHERE project_id = ?",
            vec![Value::String(project_id.to_string())],
        )?;
        
        let mut documents = Vec::new();
        for row in rows {
            if row.len() < 6 {
                continue;
            }
            
            let id = row[0].as_str().unwrap_or_default().to_string();
            let project_id = row[1].as_str().unwrap_or_default().to_string();
            let document_id = row[2].as_str().unwrap_or_default().to_string();
            let chunk_index = row[3].as_i64().unwrap_or(0) as i32;
            let content = row[4].as_str().unwrap_or_default().to_string();
            
            let metadata_str = row[5].as_str().unwrap_or("{}");
            let metadata: HashMap<String, String> = serde_json::from_str(metadata_str).unwrap_or_default();
            
            documents.push(VectorDocument {
                id,
                project_id,
                document_id,
                chunk_index,
                content,
                embedding: vec![], // Empty vector - not needed for this query
                metadata,
            });
        }
        
        // Sort documents by document_id and chunk_index in memory
        documents.sort_by(|a, b| {
            match a.document_id.cmp(&b.document_id) {
                std::cmp::Ordering::Equal => a.chunk_index.cmp(&b.chunk_index),
                other => other,
            }
        });
        
        Ok(documents)
    }
    
    /// Delete all documents for a project
    pub fn delete_project_documents(&mut self, project_id: &str) -> Result<usize> {
        let subprocess = self.subprocess.lock().unwrap();
        
        let count = subprocess.execute(
            "DELETE FROM vector_documents WHERE project_id = ?",
            vec![Value::String(project_id.to_string())],
        )?;
        
        subprocess.commit()?;
        Ok(count as usize)
    }
    
    /// Delete a specific document
    pub fn delete_document(&mut self, document_id: &str) -> Result<usize> {
        let subprocess = self.subprocess.lock().unwrap();
        
        let count = subprocess.execute(
            "DELETE FROM vector_documents WHERE document_id = ?",
            vec![Value::String(document_id.to_string())],
        )?;
        
        subprocess.commit()?;
        Ok(count as usize)
    }
    
    /// Get database statistics
    pub fn get_stats(&self) -> Result<HashMap<String, i64>> {
        let subprocess = self.subprocess.lock().unwrap();
        let mut stats = HashMap::new();
        
        // Total documents
        if let Some(row) = subprocess.query_one("SELECT COUNT(*) FROM vector_documents", vec![])? {
            if let Some(count) = row[0].as_i64() {
                stats.insert("total_documents".to_string(), count);
            }
        }
        
        // Total projects
        if let Some(row) = subprocess.query_one(
            "SELECT COUNT(DISTINCT project_id) FROM vector_documents",
            vec![],
        )? {
            if let Some(count) = row[0].as_i64() {
                stats.insert("total_projects".to_string(), count);
            }
        }
        
        Ok(stats)
    }
    
    /// Count documents in a project
    pub fn count_project_documents(&self, project_id: &str) -> Result<usize> {
        let subprocess = self.subprocess.lock().unwrap();
        
        if let Some(row) = subprocess.query_one(
            "SELECT COUNT(DISTINCT document_id) FROM vector_documents WHERE project_id = ?",
            vec![Value::String(project_id.to_string())],
        )? {
            if let Some(count) = row[0].as_i64() {
                return Ok(count as usize);
            }
        }
        
        Ok(0)
    }
    
    /// Save project to database
    pub fn save_project(&mut self, project: &crate::models::project::Project) -> Result<()> {
        log::info!("💾 [SAVE-PROJECT] Saving project: id={}, name={}", project.id, project.name);
        
        let subprocess = self.subprocess.lock().unwrap();
        
        subprocess.execute(
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
        )?;
        
        subprocess.commit()?;
        log::info!("💾 [SAVE-PROJECT] Project saved successfully");
        Ok(())
    }
    
    /// Load all projects from database
    pub fn load_all_projects(&self) -> Result<Vec<crate::models::project::Project>> {
        use chrono::DateTime;
        use uuid::Uuid;
        
        let subprocess = self.subprocess.lock().unwrap();
        
        // Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
        let rows = subprocess.query(
            "SELECT id, name, description, status, document_count, created_at, updated_at
             FROM projects",
            vec![],
        )?;
        
        let mut projects = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if row.len() < 7 {
                log::warn!("跳过项目 #{}: 列数不足 ({})", idx, row.len());
                continue;
            }
            
            // 解析 ID
            let id_str = row[0].as_str().unwrap_or_default();
            if id_str.is_empty() {
                log::warn!("跳过项目 #{}: ID 为空", idx);
                continue;
            }
            
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(e) => {
                    log::warn!("跳过项目 #{}: ID 解析失败 '{}': {}", idx, id_str, e);
                    continue;
                }
            };
            
            let name = row[1].as_str().unwrap_or_default().to_string();
            let description = row[2].as_str().and_then(|s| {
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            
            let status_str = row[3].as_str().unwrap_or("Created");
            let status = match status_str {
                "Created" => crate::models::project::ProjectStatus::Created,
                "Processing" => crate::models::project::ProjectStatus::Processing,
                "Ready" => crate::models::project::ProjectStatus::Ready,
                "Error" => crate::models::project::ProjectStatus::Error,
                _ => crate::models::project::ProjectStatus::Created,
            };
            
            let document_count = row[4].as_i64().unwrap_or(0) as u32;
            
            // 解析创建时间 - 添加更好的错误处理
            let created_at_str = row[5].as_str().unwrap_or_default();
            let created_at = if created_at_str.is_empty() {
                log::warn!("项目 {} '{}': 创建时间为空，使用当前时间", id, name);
                chrono::Utc::now()
            } else {
                match DateTime::parse_from_rfc3339(created_at_str) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(e) => {
                        log::warn!("项目 {} '{}': 创建时间解析失败 '{}': {}，使用当前时间", 
                            id, name, created_at_str, e);
                        chrono::Utc::now()
                    }
                }
            };
            
            // 解析更新时间 - 添加更好的错误处理
            let updated_at_str = row[6].as_str().unwrap_or_default();
            let updated_at = if updated_at_str.is_empty() {
                log::warn!("项目 {} '{}': 更新时间为空，使用创建时间", id, name);
                created_at
            } else {
                match DateTime::parse_from_rfc3339(updated_at_str) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(e) => {
                        log::warn!("项目 {} '{}': 更新时间解析失败 '{}': {}，使用创建时间", 
                            id, name, updated_at_str, e);
                        created_at
                    }
                }
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
        
        log::info!("成功加载 {} 个项目", projects.len());
        
        // Sort by updated_at DESC in memory
        projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(projects)
    }
    
    /// Delete project by ID
    pub fn delete_project_by_id(&mut self, project_id: &str) -> Result<usize> {
        let subprocess = self.subprocess.lock().unwrap();
        
        let count = subprocess.execute(
            "DELETE FROM projects WHERE id = ?",
            vec![Value::String(project_id.to_string())],
        )?;
        
        subprocess.commit()?;
        Ok(count as usize)
    }
    
    /// Update project document count
    pub fn update_project_document_count(&mut self, project_id: &str, count: u32) -> Result<()> {
        let subprocess = self.subprocess.lock().unwrap();
        
        subprocess.execute(
            "UPDATE projects SET document_count = ?, updated_at = NOW() WHERE id = ?",
            vec![
                Value::Number((count as i64).into()),
                Value::String(project_id.to_string()),
            ],
        )?;
        
        subprocess.commit()?;
        Ok(())
    }
    
    // ==================== Conversation Management ====================
    
    /// Save conversation to database
    pub fn save_conversation(&mut self, conversation: &crate::models::conversation::Conversation) -> Result<()> {
        log::info!("💾 [SAVE-CONV] Saving conversation: id={}", conversation.id);
        
        let subprocess = self.subprocess.lock().unwrap();
        
        subprocess.execute(
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
        )?;
        
        subprocess.commit()?;
        log::info!("💾 [SAVE-CONV] Conversation saved successfully");
        Ok(())
    }
    
    /// Load conversations by project
    pub fn load_conversations_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<crate::models::conversation::Conversation>> {
        use chrono::DateTime;
        use uuid::Uuid;
        
        let subprocess = self.subprocess.lock().unwrap();
        
        // Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
        let rows = subprocess.query(
            "SELECT id, project_id, title, created_at, updated_at, message_count
             FROM conversations
             WHERE project_id = ?",
            vec![Value::String(project_id.to_string())],
        )?;
        
        let mut conversations = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if row.len() < 6 {
                log::warn!("跳过对话 #{}: 列数不足 ({})", idx, row.len());
                continue;
            }
            
            // 解析 ID
            let id_str = row[0].as_str().unwrap_or_default();
            if id_str.is_empty() {
                log::warn!("跳过对话 #{}: ID 为空", idx);
                continue;
            }
            
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(e) => {
                    log::warn!("跳过对话 #{}: ID 解析失败 '{}': {}", idx, id_str, e);
                    continue;
                }
            };
            
            // 解析项目 ID
            let project_id_str = row[1].as_str().unwrap_or_default();
            let project_id = match Uuid::parse_str(project_id_str) {
                Ok(pid) => pid,
                Err(e) => {
                    log::warn!("跳过对话 {}: 项目ID 解析失败 '{}': {}", id, project_id_str, e);
                    continue;
                }
            };
            
            let title = row[2].as_str().unwrap_or_default().to_string();
            
            // 解析创建时间
            let created_at_str = row[3].as_str().unwrap_or_default();
            let created_at = if created_at_str.is_empty() {
                log::warn!("对话 {} '{}': 创建时间为空，使用当前时间", id, title);
                chrono::Utc::now()
            } else {
                match DateTime::parse_from_rfc3339(created_at_str) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(e) => {
                        log::warn!("对话 {} '{}': 创建时间解析失败 '{}': {}，使用当前时间", 
                            id, title, created_at_str, e);
                        chrono::Utc::now()
                    }
                }
            };
            
            // 解析更新时间
            let updated_at_str = row[4].as_str().unwrap_or_default();
            let updated_at = if updated_at_str.is_empty() {
                log::warn!("对话 {} '{}': 更新时间为空，使用创建时间", id, title);
                created_at
            } else {
                match DateTime::parse_from_rfc3339(updated_at_str) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(e) => {
                        log::warn!("对话 {} '{}': 更新时间解析失败 '{}': {}，使用创建时间", 
                            id, title, updated_at_str, e);
                        created_at
                    }
                }
            };
            
            let message_count = row[5].as_i64().unwrap_or(0) as u32;
            
            conversations.push(crate::models::conversation::Conversation {
                id,
                project_id,
                title,
                created_at,
                updated_at,
                message_count,
            });
        }
        
        // Sort by updated_at DESC in memory
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(conversations)
    }
    
    /// Load all conversations
    pub fn load_all_conversations(&self) -> Result<Vec<crate::models::conversation::Conversation>> {
        use chrono::DateTime;
        use uuid::Uuid;
        
        let subprocess = self.subprocess.lock().unwrap();
        
        // Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
        let rows = subprocess.query(
            "SELECT id, project_id, title, created_at, updated_at, message_count
             FROM conversations",
            vec![],
        )?;
        
        let mut conversations = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if row.len() < 6 {
                log::warn!("跳过对话 #{}: 列数不足 ({})", idx, row.len());
                continue;
            }
            
            // 解析 ID
            let id_str = row[0].as_str().unwrap_or_default();
            if id_str.is_empty() {
                log::warn!("跳过对话 #{}: ID 为空", idx);
                continue;
            }
            
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(e) => {
                    log::warn!("跳过对话 #{}: ID 解析失败 '{}': {}", idx, id_str, e);
                    continue;
                }
            };
            
            // 解析项目 ID
            let project_id_str = row[1].as_str().unwrap_or_default();
            if project_id_str.is_empty() {
                log::warn!("跳过对话 {} : 项目ID 为空", id);
                continue;
            }
            
            let project_id = match Uuid::parse_str(project_id_str) {
                Ok(pid) => pid,
                Err(e) => {
                    log::warn!("跳过对话 {}: 项目ID 解析失败 '{}': {}", id, project_id_str, e);
                    continue;
                }
            };
            
            let title = row[2].as_str().unwrap_or_default().to_string();
            
            // 解析创建时间 - 添加更好的错误处理
            let created_at_str = row[3].as_str().unwrap_or_default();
            let created_at = if created_at_str.is_empty() {
                log::warn!("对话 {} '{}': 创建时间为空，使用当前时间", id, title);
                chrono::Utc::now()
            } else {
                match DateTime::parse_from_rfc3339(created_at_str) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(e) => {
                        log::warn!("对话 {} '{}': 创建时间解析失败 '{}': {}，使用当前时间", 
                            id, title, created_at_str, e);
                        chrono::Utc::now()
                    }
                }
            };
            
            // 解析更新时间 - 添加更好的错误处理
            let updated_at_str = row[4].as_str().unwrap_or_default();
            let updated_at = if updated_at_str.is_empty() {
                log::warn!("对话 {} '{}': 更新时间为空，使用创建时间", id, title);
                created_at
            } else {
                match DateTime::parse_from_rfc3339(updated_at_str) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(e) => {
                        log::warn!("对话 {} '{}': 更新时间解析失败 '{}': {}，使用创建时间", 
                            id, title, updated_at_str, e);
                        created_at
                    }
                }
            };
            
            let message_count = row[5].as_i64().unwrap_or(0) as u32;
            
            conversations.push(crate::models::conversation::Conversation {
                id,
                project_id,
                title,
                created_at,
                updated_at,
                message_count,
            });
        }
        
        log::info!("成功加载 {} 个对话", conversations.len());
        
        // Sort by updated_at DESC in memory
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(conversations)
    }
    
    /// Delete conversation by ID
    pub fn delete_conversation_by_id(&mut self, conversation_id: &str) -> Result<usize> {
        let subprocess = self.subprocess.lock().unwrap();
        
        let count = subprocess.execute(
            "DELETE FROM conversations WHERE id = ?",
            vec![Value::String(conversation_id.to_string())],
        )?;
        
        subprocess.commit()?;
        Ok(count as usize)
    }
    
    /// Delete message by ID
    pub fn delete_message_by_id(&mut self, message_id: &str) -> Result<usize> {
        let subprocess = self.subprocess.lock().unwrap();
        
        let count = subprocess.execute(
            "DELETE FROM messages WHERE id = ?",
            vec![Value::String(message_id.to_string())],
        )?;
        
        subprocess.commit()?;
        Ok(count as usize)
    }
    
    /// Delete all messages in a conversation
    pub fn delete_messages_by_conversation(&mut self, conversation_id: &str) -> Result<usize> {
        let subprocess = self.subprocess.lock().unwrap();
        
        let count = subprocess.execute(
            "DELETE FROM messages WHERE conversation_id = ?",
            vec![Value::String(conversation_id.to_string())],
        )?;
        
        subprocess.commit()?;
        Ok(count as usize)
    }
    
    /// Save message to database
    pub fn save_message(&mut self, message: &crate::models::conversation::Message) -> Result<()> {
        log::info!("📝 [SAVE-MSG] Saving message: id={}", message.id);
        
        let subprocess = self.subprocess.lock().unwrap();
        
        let sources_json = message.sources.as_ref()
            .map(|s| serde_json::to_string(s).ok())
            .flatten();
        
        // 尝试 INSERT
        let insert_result = subprocess.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at, sources)
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(message.id.to_string()),
                Value::String(message.conversation_id.to_string()),
                Value::String(message.role.to_string()),
                Value::String(message.content.clone()),
                Value::String(message.timestamp.to_rfc3339()),
                sources_json.clone().map(Value::String).unwrap_or(Value::Null),
            ],
        );
        
        // 如果 INSERT 失败（主键冲突），尝试 UPDATE
        match insert_result {
            Ok(_) => {
                log::info!("✅ [SAVE-MSG] INSERT 成功");
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Duplicated primary key") || error_msg.contains("1062") {
                    log::info!("💡 [SAVE-MSG] 主键已存在，尝试 UPDATE");
                    subprocess.execute(
                        "UPDATE messages SET role=?, content=?, created_at=?, sources=? WHERE id=?",
                        vec![
                            Value::String(message.role.to_string()),
                            Value::String(message.content.clone()),
                            Value::String(message.timestamp.to_rfc3339()),
                            sources_json.map(Value::String).unwrap_or(Value::Null),
                            Value::String(message.id.to_string()),
                        ],
                    )?;
                    log::info!("✅ [SAVE-MSG] UPDATE 成功");
                } else {
                    log::error!("❌ [SAVE-MSG] INSERT 失败: {}", e);
                    return Err(e);
                }
            }
        }
        
        subprocess.commit()?;
        log::info!("📝 [SAVE-MSG] Message saved successfully");
        Ok(())
    }
    
    /// Get message count
    pub fn get_message_count(&self) -> Result<i32> {
        let subprocess = self.subprocess.lock().unwrap();
        
        if let Some(row) = subprocess.query_one("SELECT COUNT(*) FROM messages", vec![])? {
            if let Some(count) = row[0].as_i64() {
                return Ok(count as i32);
            }
        }
        
        Ok(0)
    }
    
    /// Get conversation message count
    pub fn get_conversation_message_count(&self, conversation_id: &str) -> Result<i32> {
        let subprocess = self.subprocess.lock().unwrap();
        
        if let Some(row) = subprocess.query_one(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ?",
            vec![Value::String(conversation_id.to_string())],
        )? {
            if let Some(count) = row[0].as_i64() {
                return Ok(count as i32);
            }
        }
        
        Ok(0)
    }
    
    /// Load messages by conversation
    pub fn load_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<crate::models::conversation::Message>> {
        use chrono::DateTime;
        use uuid::Uuid;
        
        let subprocess = self.subprocess.lock().unwrap();
        
        // Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
        let rows = subprocess.query(
            "SELECT id, conversation_id, role, content, created_at, sources
             FROM messages
             WHERE conversation_id = ?",
            vec![Value::String(conversation_id.to_string())],
        )?;
        
        let mut messages = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if row.len() < 6 {
                log::warn!("跳过消息 #{}: 列数不足 ({})", idx, row.len());
                continue;
            }
            
            // 解析消息 ID
            let id_str = row[0].as_str().unwrap_or_default();
            if id_str.is_empty() {
                log::warn!("跳过消息 #{}: ID 为空", idx);
                continue;
            }
            
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(e) => {
                    log::warn!("跳过消息 #{}: ID 解析失败 '{}': {}", idx, id_str, e);
                    continue;
                }
            };
            
            // 解析对话 ID
            let conversation_id_str = row[1].as_str().unwrap_or_default();
            let conversation_id = match Uuid::parse_str(conversation_id_str) {
                Ok(cid) => cid,
                Err(e) => {
                    log::warn!("跳过消息 {}: 对话ID 解析失败 '{}': {}", id, conversation_id_str, e);
                    continue;
                }
            };
            
            let role_str = row[2].as_str().unwrap_or("User");
            let role = match role_str {
                "User" | "user" => crate::models::conversation::MessageRole::User,
                "Assistant" | "assistant" => crate::models::conversation::MessageRole::Assistant,
                "System" | "system" => crate::models::conversation::MessageRole::System,
                _ => crate::models::conversation::MessageRole::User,
            };
            
            let content = row[3].as_str().unwrap_or_default().to_string();
            
            // 解析创建时间
            let created_at_str = row[4].as_str().unwrap_or_default();
            let created_at = if created_at_str.is_empty() {
                log::warn!("消息 {}: 创建时间为空，使用当前时间", id);
                chrono::Utc::now()
            } else {
                match DateTime::parse_from_rfc3339(created_at_str) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(e) => {
                        log::warn!("消息 {}: 创建时间解析失败 '{}': {}，使用当前时间", 
                            id, created_at_str, e);
                        chrono::Utc::now()
                    }
                }
            };
            
            let sources = row[5].as_str()
                .and_then(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        serde_json::from_str(s).ok()
                    }
                });
            
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
        
        // Sort by created_at ASC in memory
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        Ok(messages)
    }
    
    /// Verify database connection by running a simple query
    pub fn verify_connection(&self) -> Result<()> {
        log::info!("🔍 验证 SeekDB 数据库连接...");
        
        let subprocess = self.subprocess.lock().unwrap();
        
        // Try to execute a simple query
        match subprocess.query("SELECT 1", vec![]) {
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
    
    /// Health check - ping subprocess and verify connection
    pub fn health_check(&self) -> Result<()> {
        log::info!("🏥 执行 SeekDB 健康检查...");
        
        // Check if subprocess is alive
        let subprocess = self.subprocess.lock().unwrap();
        subprocess.ping()
            .map_err(|e| anyhow!("Python 子进程无响应: {}", e))?;
        
        drop(subprocess);
        
        // Verify database connection
        self.verify_connection()?;
        
        log::info!("✅ SeekDB 健康检查通过");
        Ok(())
    }
}

// No Drop implementation needed - Python subprocess manager handles cleanup

