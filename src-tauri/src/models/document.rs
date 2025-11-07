use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProcessingStatus {
    Uploaded,
    Processing,
    Indexed,
    Failed,
}

impl std::fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingStatus::Uploaded => write!(f, "Uploaded"),
            ProcessingStatus::Processing => write!(f, "Processing"),
            ProcessingStatus::Indexed => write!(f, "Indexed"),
            ProcessingStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub project_id: Uuid,
    pub filename: String,
    pub file_path: String,
    pub file_size: u64,
    pub mime_type: String,
    pub content_hash: String,
    pub chunk_count: u32,
    pub processing_status: ProcessingStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

impl Document {
    pub fn new(
        project_id: Uuid,
        file_path: String,
        file_size: u64,
        content_hash: String,
    ) -> Result<Self, DocumentValidationError> {
        let path = Path::new(&file_path);
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(DocumentValidationError::InvalidFilePath)?
            .to_string();

        Self::validate_filename(&filename)?;
        Self::validate_file_size(file_size)?;

        let mime_type = Self::detect_mime_type(&filename)?;

        Ok(Document {
            id: Uuid::new_v4(),
            project_id,
            filename,
            file_path,
            file_size,
            mime_type,
            content_hash,
            chunk_count: 0,
            processing_status: ProcessingStatus::Uploaded,
            error_message: None,
            created_at: Utc::now(),
            processed_at: None,
        })
    }

    pub fn update_processing_status(&mut self, status: ProcessingStatus, error_message: Option<String>) {
        self.processing_status = status.clone();
        self.error_message = error_message;

        if matches!(status, ProcessingStatus::Indexed | ProcessingStatus::Failed) {
            self.processed_at = Some(Utc::now());
        }
    }

    pub fn update_chunk_count(&mut self, count: u32) {
        self.chunk_count = count;
    }

    fn validate_filename(filename: &str) -> Result<(), DocumentValidationError> {
        if filename.trim().is_empty() {
            return Err(DocumentValidationError::EmptyFilename);
        }
        if filename.len() > 255 {
            return Err(DocumentValidationError::FilenameTooLong);
        }
        Ok(())
    }

    fn validate_file_size(size: u64) -> Result<(), DocumentValidationError> {
        const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB
        if size > MAX_FILE_SIZE {
            return Err(DocumentValidationError::FileTooLarge);
        }
        if size == 0 {
            return Err(DocumentValidationError::EmptyFile);
        }
        Ok(())
    }

    fn detect_mime_type(filename: &str) -> Result<String, DocumentValidationError> {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "txt" => Ok("text/plain".to_string()),
            "md" | "markdown" => Ok("text/markdown".to_string()),
            "pdf" => Ok("application/pdf".to_string()),
            _ => Err(DocumentValidationError::UnsupportedFileType(extension)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chunk_index: u32,
    pub content: String,
    pub token_count: u32,
    pub start_offset: u64,
    pub end_offset: u64,
    pub embedding_id: String,
    pub created_at: DateTime<Utc>,
}

impl DocumentChunk {
    pub fn new(
        document_id: Uuid,
        chunk_index: u32,
        content: String,
        start_offset: u64,
        end_offset: u64,
    ) -> Result<Self, DocumentValidationError> {
        Self::validate_content(&content)?;
        Self::validate_offsets(start_offset, end_offset)?;

        let token_count = Self::estimate_token_count(&content);
        Self::validate_token_count(token_count)?;

        Ok(DocumentChunk {
            id: Uuid::new_v4(),
            document_id,
            chunk_index,
            content,
            token_count,
            start_offset,
            end_offset,
            embedding_id: String::new(), // Will be set when stored in vector DB
            created_at: Utc::now(),
        })
    }

    pub fn set_embedding_id(&mut self, embedding_id: String) {
        self.embedding_id = embedding_id;
    }

    fn validate_content(content: &str) -> Result<(), DocumentValidationError> {
        if content.trim().is_empty() {
            return Err(DocumentValidationError::EmptyChunkContent);
        }
        Ok(())
    }

    fn validate_offsets(start: u64, end: u64) -> Result<(), DocumentValidationError> {
        if start >= end {
            return Err(DocumentValidationError::InvalidOffsets);
        }
        Ok(())
    }

    fn validate_token_count(count: u32) -> Result<(), DocumentValidationError> {
        if count < 10 || count > 1000 {
            return Err(DocumentValidationError::InvalidTokenCount);
        }
        Ok(())
    }

    fn estimate_token_count(content: &str) -> u32 {
        // Simple token estimation: roughly 4 characters per token
        (content.len() as f32 / 4.0).ceil() as u32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub filename: String,
    pub file_size: u64,
    pub processing_status: String,
    pub created_at: String,
    pub error_message: Option<String>,
}

impl From<Document> for DocumentResponse {
    fn from(document: Document) -> Self {
        DocumentResponse {
            id: document.id.to_string(),
            filename: document.filename,
            file_size: document.file_size,
            processing_status: document.processing_status.to_string(),
            created_at: document.created_at.to_rfc3339(),
            error_message: document.error_message,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DocumentValidationError {
    #[error("Invalid file path")]
    InvalidFilePath,
    #[error("Filename cannot be empty")]
    EmptyFilename,
    #[error("Filename cannot exceed 255 characters")]
    FilenameTooLong,
    #[error("File is too large (maximum 50MB)")]
    FileTooLarge,
    #[error("File cannot be empty")]
    EmptyFile,
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("Chunk content cannot be empty")]
    EmptyChunkContent,
    #[error("Invalid offsets: start must be less than end")]
    InvalidOffsets,
    #[error("Token count must be between 10 and 1000")]
    InvalidTokenCount,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let project_id = Uuid::new_v4();
        let document = Document::new(
            project_id,
            "/path/to/test.txt".to_string(),
            1024,
            "hash123".to_string(),
        );

        assert!(document.is_ok());
        let document = document.unwrap();
        assert_eq!(document.project_id, project_id);
        assert_eq!(document.filename, "test.txt");
        assert_eq!(document.mime_type, "text/plain");
        assert_eq!(document.processing_status, ProcessingStatus::Uploaded);
    }

    #[test]
    fn test_document_validation() {
        let project_id = Uuid::new_v4();

        // Test file too large
        let result = Document::new(project_id, "/path/to/large.txt".to_string(), 100 * 1024 * 1024, "hash".to_string());
        assert!(result.is_err());

        // Test unsupported file type
        let result = Document::new(project_id, "/path/to/file.exe".to_string(), 1024, "hash".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_document_chunk_creation() {
        let document_id = Uuid::new_v4();
        let chunk = DocumentChunk::new(
            document_id,
            0,
            "This is a test chunk with enough content to be valid.".to_string(),
            0,
            50,
        );

        assert!(chunk.is_ok());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.document_id, document_id);
        assert_eq!(chunk.chunk_index, 0);
        assert!(chunk.token_count >= 10);
    }

    #[test]
    fn test_chunk_validation() {
        let document_id = Uuid::new_v4();

        // Test empty content
        let result = DocumentChunk::new(document_id, 0, "".to_string(), 0, 10);
        assert!(result.is_err());

        // Test invalid offsets
        let result = DocumentChunk::new(document_id, 0, "Valid content".to_string(), 10, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(Document::detect_mime_type("test.txt").unwrap(), "text/plain");
        assert_eq!(Document::detect_mime_type("test.md").unwrap(), "text/markdown");
        assert_eq!(Document::detect_mime_type("test.pdf").unwrap(), "application/pdf");
        assert!(Document::detect_mime_type("test.exe").is_err());
    }
}
