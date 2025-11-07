use crate::models::{document::DocumentChunk, conversation::ContextChunk};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct VectorDbService {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub documents: Vec<String>,
    pub metadatas: Option<Vec<HashMap<String, Value>>>,
    pub ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query_texts: Vec<String>,
    pub n_results: Option<usize>,
    pub where_clause: Option<HashMap<String, Value>>,
    pub include: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub ids: Vec<Vec<String>>,
    pub distances: Vec<Vec<f64>>,
    pub metadatas: Option<Vec<Vec<HashMap<String, Value>>>>,
    pub documents: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub name: String,
    pub count: usize,
}

impl VectorDbService {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("http://{}:{}/api/v1", host, port),
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/heartbeat", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn create_collection(&self, collection_name: &str) -> Result<()> {
        let url = format!("{}/collections", self.base_url);

        let payload = json!({
            "name": collection_name,
            "metadata": {
                "description": "Knowledge base document embeddings"
            }
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow!("Failed to create collection: {}", error_text))
        }
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);

        let response = self.client
            .delete(&url)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow!("Failed to delete collection: {}", error_text))
        }
    }

    pub async fn collection_exists(&self, collection_name: &str) -> Result<bool> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn add_documents(&self, collection_name: &str, chunks: &[DocumentChunk]) -> Result<Vec<String>> {
        let url = format!("{}/collections/{}/add", self.base_url, collection_name);

        let documents: Vec<String> = chunks.iter().map(|chunk| chunk.content.clone()).collect();
        let ids: Vec<String> = chunks.iter().map(|chunk| chunk.id.to_string()).collect();
        let metadatas: Vec<HashMap<String, Value>> = chunks.iter().map(|chunk| {
            let mut metadata = HashMap::new();
            metadata.insert("document_id".to_string(), json!(chunk.document_id.to_string()));
            metadata.insert("chunk_index".to_string(), json!(chunk.chunk_index));
            metadata.insert("token_count".to_string(), json!(chunk.token_count));
            metadata.insert("start_offset".to_string(), json!(chunk.start_offset));
            metadata.insert("end_offset".to_string(), json!(chunk.end_offset));
            metadata.insert("created_at".to_string(), json!(chunk.created_at.to_rfc3339()));
            metadata
        }).collect();

        let payload = EmbeddingRequest {
            documents,
            metadatas: Some(metadatas),
            ids: ids.clone(),
        };

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(ids)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow!("Failed to add documents: {}", error_text))
        }
    }

    pub async fn query_documents(
        &self,
        collection_name: &str,
        query_text: &str,
        limit: usize,
        project_filter: Option<Uuid>,
    ) -> Result<Vec<ContextChunk>> {
        let url = format!("{}/collections/{}/query", self.base_url, collection_name);

        let mut where_clause = HashMap::new();
        if let Some(project_id) = project_filter {
            where_clause.insert("project_id".to_string(), json!(project_id.to_string()));
        }

        let payload = QueryRequest {
            query_texts: vec![query_text.to_string()],
            n_results: Some(limit),
            where_clause: if where_clause.is_empty() { None } else { Some(where_clause) },
            include: Some(vec!["documents".to_string(), "metadatas".to_string(), "distances".to_string()]),
        };

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to query documents: {}", error_text));
        }

        let query_response: QueryResponse = response.json().await?;

        let mut context_chunks = Vec::new();

        if let (Some(documents), Some(metadatas), Some(distances)) = (
            query_response.documents.as_ref().and_then(|d| d.get(0)),
            query_response.metadatas.as_ref().and_then(|m| m.get(0)),
            query_response.distances.get(0),
        ) {
            for (i, (document, metadata)) in documents.iter().zip(metadatas.iter()).enumerate() {
                let distance = distances.get(i).unwrap_or(&1.0);
                let relevance_score = 1.0 - distance; // Convert distance to relevance score

                let document_id = metadata
                    .get("document_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let filename = metadata
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                context_chunks.push(ContextChunk {
                    document_id: document_id.to_string(),
                    filename: filename.to_string(),
                    content: document.clone(),
                    relevance_score,
                });
            }
        }

        // Sort by relevance score (highest first)
        context_chunks.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(context_chunks)
    }

    pub async fn delete_documents(&self, collection_name: &str, document_ids: &[String]) -> Result<()> {
        let url = format!("{}/collections/{}/delete", self.base_url, collection_name);

        let payload = json!({
            "ids": document_ids
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow!("Failed to delete documents: {}", error_text))
        }
    }

    pub async fn get_collection_info(&self, collection_name: &str) -> Result<CollectionInfo> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get collection info: {}", error_text));
        }

        let collection_data: Value = response.json().await?;

        let name = collection_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(collection_name)
            .to_string();

        let count = collection_data
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        Ok(CollectionInfo { name, count })
    }

    pub fn get_collection_name_for_project(project_id: Uuid) -> String {
        format!("project_{}", project_id.to_string().replace('-', "_"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::document::DocumentChunk;
    use chrono::Utc;

    #[test]
    fn test_collection_name_generation() {
        let project_id = Uuid::new_v4();
        let collection_name = VectorDbService::get_collection_name_for_project(project_id);

        assert!(collection_name.starts_with("project_"));
        assert!(!collection_name.contains('-')); // Hyphens should be replaced with underscores
    }

    #[test]
    fn test_vector_db_service_creation() {
        let service = VectorDbService::new("localhost", 8000);
        assert_eq!(service.base_url, "http://localhost:8000/api/v1");
    }

    #[tokio::test]
    async fn test_embedding_request_serialization() {
        let request = EmbeddingRequest {
            documents: vec!["Test document".to_string()],
            metadatas: None,
            ids: vec!["test_id".to_string()],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test document"));
        assert!(json.contains("test_id"));
    }

    #[test]
    fn test_context_chunk_creation() {
        let chunk = ContextChunk {
            document_id: "doc_1".to_string(),
            filename: "test.txt".to_string(),
            content: "Test content".to_string(),
            relevance_score: 0.95,
        };

        assert_eq!(chunk.document_id, "doc_1");
        assert_eq!(chunk.relevance_score, 0.95);
    }

    // Note: Integration tests with actual Chroma instance would go in tests/rust/integration/
}
