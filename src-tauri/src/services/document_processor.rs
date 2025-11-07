use crate::models::document::{Document, DocumentChunk};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DocumentProcessor {
    max_chunk_size: usize,
    chunk_overlap: usize,
}

#[derive(Debug, Clone)]
pub struct ProcessingResult {
    pub chunks: Vec<DocumentChunk>,
    pub total_tokens: u32,
    pub processing_time: f64,
}

impl DocumentProcessor {
    pub fn new() -> Self {
        Self {
            max_chunk_size: 1000, // tokens
            chunk_overlap: 100,   // tokens
        }
    }

    pub fn with_chunk_settings(max_chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            max_chunk_size,
            chunk_overlap,
        }
    }

    pub async fn process_document(&self, document: &Document) -> Result<ProcessingResult> {
        let start_time = std::time::Instant::now();

        // Read file content
        let content = self.read_file_content(&document.file_path, &document.mime_type).await?;

        // Create chunks
        let chunks = self.create_chunks(document.id, &content)?;

        let total_tokens: u32 = chunks.iter().map(|chunk| chunk.token_count).sum();
        let processing_time = start_time.elapsed().as_secs_f64();

        Ok(ProcessingResult {
            chunks,
            total_tokens,
            processing_time,
        })
    }

    async fn read_file_content(&self, file_path: &str, mime_type: &str) -> Result<String> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(anyhow!("File not found: {}", file_path));
        }

        match mime_type {
            "text/plain" | "text/markdown" => {
                let content = fs::read_to_string(path)?;
                Ok(self.clean_text(&content))
            }
            "application/pdf" => {
                self.extract_pdf_text(path).await
            }
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                self.extract_docx_text(path).await
            }
            "application/rtf" => {
                self.extract_rtf_text(path).await
            }
            _ => Err(anyhow!("Unsupported file type: {}", mime_type)),
        }
    }

    async fn extract_pdf_text(&self, path: &Path) -> Result<String> {
        // 使用pdf-extract库提取PDF文本
        match pdf_extract::extract_text(path) {
            Ok(text) => Ok(self.clean_text(&text)),
            Err(e) => Err(anyhow!("Failed to extract PDF text: {}", e)),
        }
    }

    async fn extract_docx_text(&self, path: &Path) -> Result<String> {
        // 使用docx-rs库提取DOCX文本
        let content = fs::read(path)?;
        match docx_rs::read_docx(&content) {
            Ok(docx) => {
                let mut text = String::new();
                for child in docx.document.children {
                    if let docx_rs::DocumentChild::Paragraph(p) = child {
                        for child in p.children {
                            if let docx_rs::ParagraphChild::Run(r) = child {
                                for run_child in r.children {
                                    if let docx_rs::RunChild::Text(t) = run_child {
                                        text.push_str(&t.text);
                                    }
                                }
                            }
                        }
                        text.push('\n');
                    }
                }
                Ok(self.clean_text(&text))
            }
            Err(e) => Err(anyhow!("Failed to extract DOCX text: {}", e)),
        }
    }

    async fn extract_rtf_text(&self, path: &Path) -> Result<String> {
        // 简单的RTF文本提取（移除RTF控制字符）
        let content = fs::read_to_string(path)?;
        let text = self.strip_rtf_formatting(&content);
        Ok(self.clean_text(&text))
    }

    fn strip_rtf_formatting(&self, rtf_content: &str) -> String {
        // 简单的RTF格式移除
        use regex::Regex;
        let re = Regex::new(r"\\[a-zA-Z]+\d*\s*").unwrap();
        let text = re.replace_all(rtf_content, "");

        // 移除花括号
        let re = Regex::new(r"[{}]").unwrap();
        let text = re.replace_all(&text, "");

        text.to_string()
    }

    fn clean_text(&self, text: &str) -> String {
        // 清理文本：保留换行符结构，移除每行内的多余空白
        use regex::Regex;

        // 按行分割，保留行结构（对 Markdown 表格等格式很重要）
        let lines: Vec<String> = text
            .lines()
            .map(|line| {
                // 清理每行内的多余空白（制表符、多个空格等），但保留行分隔
                let re = Regex::new(r"[ \t]+").unwrap();
                re.replace_all(line.trim(), " ").to_string()
            })
            .filter(|line| !line.is_empty()) // 移除完全空白的行
            .collect();

        // 用单个换行符重新连接，保持文档结构
        lines.join("\n")
    }

    fn create_chunks(&self, document_id: Uuid, content: &str) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        let mut current_offset = 0;
        let mut chunk_index = 0;

        // Split content into sentences for better chunking
        let sentences = self.split_into_sentences(content);
        let mut current_chunk = String::new();
        let mut current_chunk_start = 0;

        for sentence in sentences {
            let sentence_tokens = self.estimate_token_count(&sentence);
            let current_tokens = self.estimate_token_count(&current_chunk);

            // If adding this sentence would exceed max chunk size, create a chunk
            if current_tokens + sentence_tokens > self.max_chunk_size && !current_chunk.is_empty() {
                let chunk_end = current_offset;

                if let Ok(chunk) = DocumentChunk::new(
                    document_id,
                    chunk_index,
                    current_chunk.trim().to_string(),
                    current_chunk_start as u64,
                    chunk_end as u64,
                ) {
                    chunks.push(chunk);
                    chunk_index += 1;
                }

                // Start new chunk with overlap
                current_chunk = self.create_overlap_content(&current_chunk, &sentence);
                current_chunk_start = self.calculate_overlap_start(current_offset, &current_chunk);
            } else {
                if current_chunk.is_empty() {
                    current_chunk_start = current_offset;
                }
                current_chunk.push_str(&sentence);
                current_chunk.push(' ');
            }

            current_offset += sentence.len() + 1; // +1 for space
        }

        // Create final chunk if there's remaining content
        if !current_chunk.trim().is_empty() {
            if let Ok(chunk) = DocumentChunk::new(
                document_id,
                chunk_index,
                current_chunk.trim().to_string(),
                current_chunk_start as u64,
                current_offset as u64,
            ) {
                chunks.push(chunk);
            }
        }

        if chunks.is_empty() {
            return Err(anyhow!("No valid chunks could be created from document"));
        }

        Ok(chunks)
    }

    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        // Simple sentence splitting - in a real implementation, you might use
        // a more sophisticated NLP library for better sentence boundary detection
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();

        for char in text.chars() {
            current_sentence.push(char);

            // Simple sentence boundary detection
            if matches!(char, '.' | '!' | '?') {
                let trimmed = current_sentence.trim();
                if !trimmed.is_empty() && trimmed.len() > 3 {
                    sentences.push(trimmed.to_string());
                    current_sentence.clear();
                }
            }
        }

        // Add remaining text as a sentence
        let trimmed = current_sentence.trim();
        if !trimmed.is_empty() {
            sentences.push(trimmed.to_string());
        }

        // If no sentences were found, split by lines and group them
        // to ensure each chunk meets minimum token requirements (10 tokens = ~40 chars)
        if sentences.is_empty() {
            const MIN_CHUNK_CHARS: usize = 40; // Minimum characters per chunk
            let lines: Vec<&str> = text
                .split('\n')
                .filter(|line| !line.trim().is_empty())
                .collect();

            if !lines.is_empty() {
                let mut current_group = String::new();

                for line in lines {
                    if current_group.is_empty() {
                        current_group = line.trim().to_string();
                    } else {
                        // Add line to current group
                        current_group.push('\n');
                        current_group.push_str(line.trim());
                    }

                    // If group is large enough, save it and start new group
                    if current_group.len() >= MIN_CHUNK_CHARS {
                        sentences.push(current_group.clone());
                        current_group.clear();
                    }
                }

                // Don't forget remaining content
                if !current_group.is_empty() {
                    // If the last group is too small, append to previous sentence if possible
                    if current_group.len() < MIN_CHUNK_CHARS && !sentences.is_empty() {
                        let last_idx = sentences.len() - 1;
                        sentences[last_idx].push('\n');
                        sentences[last_idx].push_str(&current_group);
                    } else {
                        sentences.push(current_group);
                    }
                }
            }
        }

        sentences
    }

    fn create_overlap_content(&self, previous_chunk: &str, new_sentence: &str) -> String {
        let overlap_tokens = self.chunk_overlap;
        let words: Vec<&str> = previous_chunk.split_whitespace().collect();

        if words.len() > overlap_tokens {
            let overlap_start = words.len() - overlap_tokens;
            let overlap_text = words[overlap_start..].join(" ");
            format!("{} {}", overlap_text, new_sentence)
        } else {
            new_sentence.to_string()
        }
    }

    fn calculate_overlap_start(&self, current_offset: usize, overlap_content: &str) -> usize {
        // Estimate where the overlap content starts in the original text
        let overlap_length = overlap_content.len();
        if current_offset >= overlap_length {
            current_offset - overlap_length
        } else {
            0
        }
    }

    fn estimate_token_count(&self, text: &str) -> usize {
        // Simple token estimation: roughly 4 characters per token
        // This is a rough approximation - for production use, you'd want
        // to use a proper tokenizer like tiktoken
        (text.len() as f32 / 4.0).ceil() as usize
    }

    pub fn validate_file(&self, file_path: &str) -> Result<()> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }

        if !path.is_file() {
            return Err(anyhow!("Path is not a file: {}", file_path));
        }

        let metadata = fs::metadata(path)?;
        let file_size = metadata.len();

        // Check file size (50MB limit)
        const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
        if file_size > MAX_FILE_SIZE {
            return Err(anyhow!("File too large: {} bytes (max: {} bytes)", file_size, MAX_FILE_SIZE));
        }

        if file_size == 0 {
            return Err(anyhow!("File is empty: {}", file_path));
        }

        Ok(())
    }

    pub fn get_supported_extensions() -> Vec<&'static str> {
        vec!["txt", "md", "markdown", "pdf", "doc", "docx", "rtf"]
    }

    pub fn is_supported_file(&self, file_path: &str) -> bool {
        let path = Path::new(file_path);
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            Self::get_supported_extensions().contains(&extension.to_lowercase().as_str())
        } else {
            false
        }
    }
}

impl Default for DocumentProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_document_processor_creation() {
        let processor = DocumentProcessor::new();
        assert_eq!(processor.max_chunk_size, 1000);
        assert_eq!(processor.chunk_overlap, 100);

        let processor = DocumentProcessor::with_chunk_settings(500, 50);
        assert_eq!(processor.max_chunk_size, 500);
        assert_eq!(processor.chunk_overlap, 50);
    }

    #[test]
    fn test_sentence_splitting() {
        let processor = DocumentProcessor::new();
        let text = "This is sentence one. This is sentence two! Is this sentence three?";
        let sentences = processor.split_into_sentences(text);

        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("sentence one"));
        assert!(sentences[1].contains("sentence two"));
        assert!(sentences[2].contains("sentence three"));
    }

    #[test]
    fn test_token_estimation() {
        let processor = DocumentProcessor::new();
        let text = "This is a test";
        let tokens = processor.estimate_token_count(text);

        // "This is a test" is 14 characters, so roughly 4 tokens
        assert!(tokens >= 3 && tokens <= 5);
    }

    #[test]
    fn test_supported_extensions() {
        let extensions = DocumentProcessor::get_supported_extensions();
        assert!(extensions.contains(&"txt"));
        assert!(extensions.contains(&"md"));
        assert!(extensions.contains(&"pdf"));
    }

    #[test]
    fn test_file_support_check() {
        let processor = DocumentProcessor::new();

        assert!(processor.is_supported_file("test.txt"));
        assert!(processor.is_supported_file("test.md"));
        assert!(processor.is_supported_file("test.PDF")); // Case insensitive
        assert!(!processor.is_supported_file("test.exe"));
        assert!(!processor.is_supported_file("test"));
    }

    #[tokio::test]
    async fn test_file_validation() {
        let processor = DocumentProcessor::new();

        // Test non-existent file
        let result = processor.validate_file("/non/existent/file.txt");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_text_file_processing() {
        let processor = DocumentProcessor::new();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        // Create a test file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "This is the first sentence. This is the second sentence. This is the third sentence.").unwrap();

        // Create a mock document
        let document = Document::new(
            Uuid::new_v4(),
            file_path.to_string_lossy().to_string(),
            100,
            "test_hash".to_string(),
        ).unwrap();

        let result = processor.process_document(&document).await;
        assert!(result.is_ok());

        let processing_result = result.unwrap();
        assert!(!processing_result.chunks.is_empty());
        assert!(processing_result.total_tokens > 0);
        assert!(processing_result.processing_time >= 0.0);
    }

    #[test]
    fn test_chunk_creation() {
        let processor = DocumentProcessor::with_chunk_settings(50, 10); // Small chunks for testing
        let document_id = Uuid::new_v4();
        let content = "This is a long piece of text that should be split into multiple chunks. Each chunk should have some overlap with the previous chunk. This ensures continuity when searching through the document.";

        let result = processor.create_chunks(document_id, content);
        assert!(result.is_ok());

        let chunks = result.unwrap();
        assert!(chunks.len() > 1); // Should create multiple chunks

        // Verify chunk properties
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.document_id, document_id);
            assert_eq!(chunk.chunk_index, i as u32);
            assert!(!chunk.content.is_empty());
            assert!(chunk.end_offset > chunk.start_offset);
        }
    }
}
