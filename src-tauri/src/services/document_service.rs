use crate::models::document::{Document, ProcessingStatus};
use crate::services::{
    dashscope_embedding_service::DashScopeEmbeddingService,
    document_processor::DocumentProcessor,
    seekdb_adapter::{SeekDbAdapter, VectorDocument},
};
use anyhow::{anyhow, Result};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 相似文档块结构（用于聊天上下文）
#[derive(Debug, Clone)]
pub struct SimilarChunk {
    pub document_id: String,
    pub filename: Option<String>,
    pub content: String,
    pub relevance_score: f64,
}

pub struct DocumentService {
    documents: HashMap<Uuid, Document>,
    document_processor: DocumentProcessor,
    vector_db: Arc<Mutex<SeekDbAdapter>>,
    embedding_service: Arc<DashScopeEmbeddingService>,
}

impl DocumentService {
    pub async fn new() -> Result<Self> {
        // Use in-memory path for testing/temporary usage
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("mine_kb_temp.db");
        let vector_db = Arc::new(Mutex::new(SeekDbAdapter::new(db_path)?));

        // 从环境变量读取 API Key
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .map_err(|_| anyhow!("未找到 DASHSCOPE_API_KEY 环境变量"))?;
        let embedding_service = Arc::new(DashScopeEmbeddingService::new(api_key, None)?);

        Ok(Self {
            documents: HashMap::new(),
            document_processor: DocumentProcessor::new(),
            vector_db,
            embedding_service,
        })
    }

    pub async fn with_db_path(db_path: &str) -> Result<Self> {
        let vector_db = Arc::new(Mutex::new(SeekDbAdapter::new(db_path)?));

        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .map_err(|_| anyhow!("未找到 DASHSCOPE_API_KEY 环境变量"))?;
        let embedding_service = Arc::new(DashScopeEmbeddingService::new(api_key, None)?);

        Ok(Self {
            documents: HashMap::new(),
            document_processor: DocumentProcessor::new(),
            vector_db,
            embedding_service,
        })
    }

    pub async fn with_config(
        db_path: &str,
        api_key: String,
        base_url: Option<String>
    ) -> Result<Self> {
        Self::with_full_config(db_path, api_key, base_url, None).await
    }

    pub async fn with_full_config(
        db_path: &str,
        api_key: String,
        base_url: Option<String>,
        python_path: Option<&str>
    ) -> Result<Self> {
        log::info!("🏗️  [DOC-SERVICE] 初始化DocumentService, db_path: {}", db_path);
        let vector_db = Arc::new(Mutex::new(
            SeekDbAdapter::new_with_python(db_path, python_path.unwrap_or("python3"))?
        ));
        log::info!("🏗️  [DOC-SERVICE] 数据库实例已创建");

        log::info!("🎯 使用阿里云百炼 Embedding API (text-embedding-v2)");
        let embedding_service = Arc::new(DashScopeEmbeddingService::new(api_key, base_url)?);

        Ok(Self {
            documents: HashMap::new(),
            document_processor: DocumentProcessor::new(),
            vector_db,
            embedding_service,
        })
    }

    /// 获取向量数据库的引用
    pub fn get_vector_db(&self) -> Arc<Mutex<SeekDbAdapter>> {
        self.vector_db.clone()
    }

    pub async fn add_document(
        &mut self,
        project_id: Uuid,
        file_path: String,
        file_size: u64,
        content_hash: String,
    ) -> Result<Uuid> {
        // Validate file before processing
        self.document_processor.validate_file(&file_path)?;

        // Create document
        let document = Document::new(project_id, file_path, file_size, content_hash)?;
        let document_id = document.id;

        // Store document
        self.documents.insert(document_id, document.clone());

        // Process document and create embeddings
        self.process_document_async(document_id).await?;

        Ok(document_id)
    }

    async fn process_document_async(&mut self, document_id: Uuid) -> Result<()> {
        let document = self.documents.get_mut(&document_id)
            .ok_or_else(|| anyhow!("Document not found: {}", document_id))?;

        // Update status to processing
        document.processing_status = ProcessingStatus::Processing;

        // Process the document
        match self.document_processor.process_document(document).await {
            Ok(processing_result) => {
                log::info!("Document processed successfully: {} chunks", processing_result.chunks.len());

                // Create vector documents for each chunk
                let mut vector_docs = Vec::new();
                let chunk_count = processing_result.chunks.len();

                // 批量生成 embeddings（更高效）
                let chunk_texts: Vec<String> = processing_result.chunks
                    .iter()
                    .map(|c| c.content.clone())
                    .collect();

                let embeddings = self.embedding_service.embed_batch(&chunk_texts).await?;

                for (chunk, embedding) in processing_result.chunks.iter().zip(embeddings.iter()) {

                        let vector_doc = VectorDocument {
                            id: Uuid::new_v4().to_string(),
                            project_id: document.project_id.to_string(),
                            document_id: document.id.to_string(),
                            chunk_index: chunk.chunk_index as i32,
                            content: chunk.content.clone(),
                            embedding: embedding.clone(),
                            metadata: {
                                let mut meta = HashMap::new();
                                meta.insert("filename".to_string(), document.filename.clone());
                                meta.insert("mime_type".to_string(), document.mime_type.clone());
                                meta.insert("start_offset".to_string(), chunk.start_offset.to_string());
                                meta.insert("end_offset".to_string(), chunk.end_offset.to_string());
                                meta
                            },
                        };
                        vector_docs.push(vector_doc);
                    }

                // Store vectors in database
                {
                    let mut db = self.vector_db.lock().await;
                    db.add_documents(vector_docs)?;
                }

                // Update document status
                document.processing_status = ProcessingStatus::Indexed;
                document.chunk_count = chunk_count as u32;
                document.processed_at = Some(chrono::Utc::now());

                log::info!("Document indexed successfully: {}", document.filename);
            }
            Err(e) => {
                log::error!("Document processing failed: {}", e);
                document.processing_status = ProcessingStatus::Failed;
                document.error_message = Some(e.to_string());
                return Err(e);
            }
        }

        Ok(())
    }

    pub fn get_document(&self, document_id: Uuid) -> Option<&Document> {
        self.documents.get(&document_id)
    }

    pub fn get_document_mut(&mut self, document_id: Uuid) -> Option<&mut Document> {
        self.documents.get_mut(&document_id)
    }

    pub async fn search_documents(
        &self,
        query: &str,
        project_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<crate::services::seekdb_adapter::SearchResult>> {
        let query_embedding = self.embedding_service.embed_text(query).await?;
        let project_id_str = project_id.map(|id| id.to_string());

        let db = self.vector_db.lock().await;

        // 使用 DashScope embedding，相似度通常在 0.5-0.9 之间
        let results = db.similarity_search(
            &query_embedding,
            project_id_str.as_deref(),
            limit,
            0.5, // DashScope embedding 质量高，可以设置较高阈值
        )?;

        Ok(results)
    }

    /// 使用混合检索搜索相关文档块（向量+全文，用于聊天上下文）
    pub async fn search_similar_chunks_hybrid(
        &self,
        project_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SimilarChunk>> {
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("🔍 [HYBRID-SEARCH] 开始混合检索文档块");
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("📋 项目ID: {}", project_id);
        log::info!("💬 查询内容: {}", query);
        log::info!("📊 返回数量: {}", top_k);

        // 使用 DashScope API 生成查询向量
        log::info!("🌐 调用 DashScope Embedding API...");
        let query_embedding = self.embedding_service.embed_text(query).await?;
        log::info!("✅ 生成查询向量成功，维度: {}", query_embedding.len());

        // 从向量数据库执行混合搜索
        let db = self.vector_db.lock().await;

        log::info!("🔄 执行混合检索（语义权重=0.7）...");

        // 使用混合检索 (语义权重 0.7 表示更偏重向量相似度)
        let results = db.hybrid_search(
            query,
            &query_embedding,
            Some(project_id),
            top_k,
            0.7, // semantic boost: 0.7 表示向量检索占 70% 权重
        )?;

        log::info!("✅ 混合检索完成，找到 {} 个结果", results.len());

        // 打印所有结果的详细信息
        for (i, result) in results.iter().enumerate() {
            let preview = result.document.content.chars().take(80).collect::<String>();
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::info!("📄 结果 #{}", i + 1);
            log::info!("   🔢 分数: {:.4}", result.similarity);
            log::info!("   📝 内容预览: {}...", preview);
            log::info!("   📂 文档ID: {}", result.document.document_id);
            log::info!("   🔖 块索引: {}", result.document.chunk_index);
        }

        // 转换为 SimilarChunk
        let chunks: Vec<SimilarChunk> = results
            .iter()
            .map(|result| {
                // 从 metadata 中获取 filename
                let filename = result.document.metadata
                    .get("filename")
                    .cloned();

                log::debug!("文档 {} 的 filename: {:?}", result.document.document_id, filename);

                SimilarChunk {
                    document_id: result.document.document_id.clone(),
                    filename,
                    content: result.document.content.clone(),
                    relevance_score: result.similarity,
                }
            })
            .collect();

        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("✅ [HYBRID-SEARCH] 混合检索完成，返回 {} 个相关文档块", chunks.len());
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(chunks)
    }

    // 搜索相关文档块（用于聊天上下文）- 保留纯向量搜索作为备选
    pub async fn search_similar_chunks(
        &self,
        project_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SimilarChunk>> {
        log::info!("🔍 开始搜索相关文档块: project_id={}, query={}, top_k={}", project_id, query, top_k);

        // 使用 DashScope API 生成查询向量
        let query_embedding = self.embedding_service.embed_text(query).await?;
        log::info!("✅ 生成查询向量成功，维度: {}", query_embedding.len());

        // 从向量数据库搜索
        let db = self.vector_db.lock().await;

        log::info!("🔍 使用SeekDB向量检索，阈值=0.3");

        // 使用 DashScope embedding，相似度通常在 0.3-0.9 之间
        // 0.3 作为阈值可以获得较宽泛但相关的结果
        let results = db.similarity_search(
            &query_embedding,
            Some(project_id),
            top_k,
            0.3, // DashScope embedding: 0.3=宽泛, 0.4=中等, 0.5+=严格
        )?;

        log::info!("✅ 向量搜索完成（阈值=0.3），找到 {} 个结果", results.len());

        // 打印前几个结果的相似度分数
        for (i, result) in results.iter().take(3).enumerate() {
            log::info!("  📄 结果#{}: 相似度={:.4}, 内容预览={}",
                i + 1,
                result.similarity,
                &result.document.content.chars().take(50).collect::<String>()
            );
        }

        // 转换为 SimilarChunk
        let chunks: Vec<SimilarChunk> = results
            .iter()
            .map(|result| {
                // 从 metadata 中获取 filename
                let filename = result.document.metadata
                    .get("filename")
                    .cloned();

                log::debug!("文档 {} 的 filename: {:?}", result.document.document_id, filename);

                SimilarChunk {
                    document_id: result.document.document_id.clone(),
                    filename,
                    content: result.document.content.clone(),
                    relevance_score: result.similarity,
                }
            })
            .collect();

        Ok(chunks)
    }

    pub fn list_documents(&self, project_id: Option<Uuid>) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|doc| {
                if let Some(pid) = project_id {
                    doc.project_id == pid
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn delete_document(&mut self, document_id: Uuid) -> Result<()> {
        let _document = self.documents
            .remove(&document_id)
            .ok_or_else(|| anyhow!("Document not found: {}", document_id))?;

        // TODO: Delete from vector database
        // self.vector_db.delete_documents(&collection_name, &[document_id.to_string()]).await?;

        Ok(())
    }

    pub fn get_documents_by_status(&self, status: ProcessingStatus) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|doc| doc.processing_status == status)
            .collect()
    }

    pub fn update_document_status(
        &mut self,
        document_id: Uuid,
        status: ProcessingStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let document = self.documents
            .get_mut(&document_id)
            .ok_or_else(|| anyhow!("Document not found: {}", document_id))?;

        document.update_processing_status(status, error_message);
        Ok(())
    }

    pub async fn reprocess_document(&mut self, document_id: Uuid) -> Result<()> {
        let document = self.documents
            .get_mut(&document_id)
            .ok_or_else(|| anyhow!("Document not found: {}", document_id))?;

        // Reset status to processing
        document.update_processing_status(ProcessingStatus::Processing, None);

        // Reprocess
        self.process_document_async(document_id).await
    }


    pub async fn count_documents(&self, project_id: Option<Uuid>) -> usize {
        // 从数据库查询实际的文档数量，而不是从内存统计
        // 这样可以确保统计的是累加的总数，而不是当前批次的数量
        if let Some(pid) = project_id {
            let db = self.vector_db.lock().await;
            match db.count_project_documents(&pid.to_string()) {
                Ok(count) => count,
                Err(e) => {
                    log::error!("从数据库统计文档数量失败: {}", e);
                    // 降级到内存统计
                    self.documents
                        .values()
                        .filter(|doc| doc.project_id == pid)
                        .count()
                }
            }
        } else {
            // 如果没有指定项目，使用内存统计
            self.documents.len()
        }
    }

    pub fn get_processing_stats(&self, project_id: Option<Uuid>) -> HashMap<ProcessingStatus, usize> {
        let mut stats = HashMap::new();

        let documents = if let Some(pid) = project_id {
            self.documents.values().filter(|doc| doc.project_id == pid).collect::<Vec<_>>()
        } else {
            self.documents.values().collect::<Vec<_>>()
        };

        for document in documents {
            *stats.entry(document.processing_status.clone()).or_insert(0) += 1;
        }

        stats
    }

    pub fn is_supported_file(&self, file_path: &str) -> bool {
        self.document_processor.is_supported_file(file_path)
    }

    pub fn get_supported_extensions() -> Vec<&'static str> {
        DocumentProcessor::get_supported_extensions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> DocumentService {
        let vector_db = VectorDbService::new("localhost", 8000);
        DocumentService::new(vector_db)
    }

    #[test]
    fn test_document_service_creation() {
        let service = create_test_service();
        assert_eq!(service.documents.len(), 0);
    }

    #[tokio::test]
    async fn test_add_document() {
        let mut service = create_test_service();
        let project_id = Uuid::new_v4();

        // This would fail in a real test because the file doesn't exist
        // In a real implementation, you'd mock the file system
        let result = service.add_document(
            project_id,
            "/non/existent/file.txt".to_string(),
            1024,
            "hash123".to_string(),
        ).await;

        // Should fail because file doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_list_documents_by_project() {
        let service = create_test_service();
        let project_id = Uuid::new_v4();

        let documents = service.list_documents(Some(project_id));
        assert_eq!(documents.len(), 0);

        let all_documents = service.list_documents(None);
        assert_eq!(all_documents.len(), 0);
    }

    #[test]
    fn test_supported_file_check() {
        let service = create_test_service();

        assert!(service.is_supported_file("test.txt"));
        assert!(service.is_supported_file("test.md"));
        assert!(service.is_supported_file("test.pdf"));
        assert!(!service.is_supported_file("test.exe"));
    }

    #[test]
    fn test_processing_stats() {
        let service = create_test_service();
        let stats = service.get_processing_stats(None);
        assert!(stats.is_empty());
    }

    #[test]
    fn test_get_supported_extensions() {
        let extensions = DocumentService::get_supported_extensions();
        assert!(extensions.contains(&"txt"));
        assert!(extensions.contains(&"md"));
        assert!(extensions.contains(&"pdf"));
    }
}
