use crate::models::conversation::{ContextChunk, Message};
use crate::services::prompts;
use anyhow::{anyhow, Result};
use async_stream::stream;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct LlmClient {
    client: Client,
    config: LlmConfig,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Local,
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::OpenAI => write!(f, "OpenAI"),
            LlmProvider::Anthropic => write!(f, "Anthropic"),
            LlmProvider::Local => write!(f, "Local"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ChatDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Token(String),
    Context(Vec<ContextChunk>),
    Complete(String), // response_id
    Error(String),
}

pub type StreamResponse = Pin<Box<dyn Stream<Item = StreamEvent> + Send>>;

impl LlmClient {
    pub fn new(config: LlmConfig) -> Result<Self> {
        Self::validate_config(&config)?;

        Ok(Self {
            client: Client::new(),
            config,
        })
    }

    pub async fn test_connection(&self) -> Result<bool> {
        match self.config.provider {
            LlmProvider::OpenAI => self.test_openai_connection().await,
            LlmProvider::Anthropic => self.test_anthropic_connection().await,
            LlmProvider::Local => self.test_local_connection().await,
        }
    }

    pub async fn generate_response(
        &self,
        messages: &[Message],
        context_chunks: &[ContextChunk],
    ) -> Result<StreamResponse> {
        let start_time = Instant::now();

        // Build the conversation context
        let system_message = self.build_system_message(context_chunks);
        let mut chat_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_message,
        }];

        // Add conversation history
        for message in messages {
            chat_messages.push(ChatMessage {
                role: message.role.to_string().to_lowercase(),
                content: message.content.clone(),
            });
        }

        match self.config.provider {
            LlmProvider::OpenAI => self.generate_openai_response(chat_messages, context_chunks, start_time).await,
            LlmProvider::Anthropic => self.generate_anthropic_response(chat_messages, context_chunks, start_time).await,
            LlmProvider::Local => self.generate_local_response(chat_messages, context_chunks, start_time).await,
        }
    }

    async fn generate_openai_response(
        &self,
        messages: Vec<ChatMessage>,
        context_chunks: &[ContextChunk],
        _start_time: Instant,
    ) -> Result<StreamResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: self.config.stream,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        log::info!(
            "发送 LLM 请求: model={}, stream={}, base_url={}",
            self.config.model,
            self.config.stream,
            self.config.base_url
        );

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("发送请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            log::error!("LLM API 错误: status={}, error={}", status, error_text);
            return Err(anyhow!("LLM API 错误 ({}): {}", status, error_text));
        }

        if self.config.stream {
            // 流式响应
            log::info!("LLM 响应成功，开始流式读取");
            self.handle_streaming_response(response, context_chunks).await
        } else {
            // 非流式响应
            log::info!("LLM 响应成功，等待完整响应");
            self.handle_non_streaming_response(response, context_chunks).await
        }
    }

    async fn handle_streaming_response(
        &self,
        response: reqwest::Response,
        context_chunks: &[ContextChunk],
    ) -> Result<StreamResponse> {
        let context_chunks = context_chunks.to_vec();
        let mut byte_stream = response.bytes_stream();

        let stream = stream! {
            // First, emit context chunks
            if !context_chunks.is_empty() {
                yield StreamEvent::Context(context_chunks);
            }

            let response_id = format!("resp_{}", uuid::Uuid::new_v4());
            let mut buffer = String::new();

            // Parse SSE stream
            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&chunk_str);

                        // Process complete lines
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();

                            if line.is_empty() {
                                continue;
                            }

                            // SSE format: "data: {...}"
                            if line.starts_with("data: ") {
                                let json_str = &line[6..];

                                // Check for [DONE] signal
                                if json_str.trim() == "[DONE]" {
                                    log::debug!("收到流式结束信号");
                                    break;
                                }

                                // Parse JSON response
                                match serde_json::from_str::<ChatResponse>(json_str) {
                                    Ok(response) => {
                                        if let Some(choice) = response.choices.first() {
                                            if let Some(delta) = &choice.delta {
                                                if let Some(content) = &delta.content {
                                                    if !content.is_empty() {
                                                        log::debug!("收到 token: {}", content);
                                                        yield StreamEvent::Token(content.clone());
                                                    }
                                                }
                                            }

                                            // Check for finish
                                            if let Some(reason) = &choice.finish_reason {
                                                if reason == "stop" || reason == "length" {
                                                    log::info!("流式响应完成: {}", reason);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("解析 SSE 数据失败: {} - 原始数据: {}", e, json_str);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("读取流式数据失败: {}", e);
                        yield StreamEvent::Error(format!("读取流式数据失败: {}", e));
                        break;
                    }
                }
            }

            log::info!("流式响应处理完成");
            yield StreamEvent::Complete(response_id);
        };

        Ok(Box::pin(stream))
    }

    async fn handle_non_streaming_response(
        &self,
        response: reqwest::Response,
        context_chunks: &[ContextChunk],
    ) -> Result<StreamResponse> {
        let context_chunks = context_chunks.to_vec();

        // 读取完整响应
        let response_text = response.text().await
            .map_err(|e| anyhow!("读取响应失败: {}", e))?;

        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("解析响应失败: {}", e))?;

        let stream = stream! {
            // First, emit context chunks
            if !context_chunks.is_empty() {
                yield StreamEvent::Context(context_chunks);
            }

            // Extract content from response
            if let Some(choice) = chat_response.choices.first() {
                if let Some(message) = &choice.message {
                    log::info!("收到完整响应，长度: {}", message.content.len());
                    yield StreamEvent::Token(message.content.clone());
                }
            }

            yield StreamEvent::Complete(chat_response.id);
        };

        Ok(Box::pin(stream))
    }

    async fn generate_anthropic_response(
        &self,
        _messages: Vec<ChatMessage>,
        context_chunks: &[ContextChunk],
        _start_time: Instant,
    ) -> Result<StreamResponse> {
        // Placeholder for Anthropic implementation
        let context_chunks = context_chunks.to_vec();
        let stream = stream! {
            if !context_chunks.is_empty() {
                yield StreamEvent::Context(context_chunks);
            }
            yield StreamEvent::Token("Anthropic integration not implemented yet.".to_string());
            yield StreamEvent::Complete(format!("resp_{}", uuid::Uuid::new_v4()));
        };

        Ok(Box::pin(stream))
    }

    async fn generate_local_response(
        &self,
        _messages: Vec<ChatMessage>,
        context_chunks: &[ContextChunk],
        _start_time: Instant,
    ) -> Result<StreamResponse> {
        // Placeholder for local LLM implementation
        let context_chunks = context_chunks.to_vec();
        let stream = stream! {
            if !context_chunks.is_empty() {
                yield StreamEvent::Context(context_chunks);
            }
            yield StreamEvent::Token("Local LLM integration not implemented yet.".to_string());
            yield StreamEvent::Complete(format!("resp_{}", uuid::Uuid::new_v4()));
        };

        Ok(Box::pin(stream))
    }

    fn build_system_message(&self, context_chunks: &[ContextChunk]) -> String {
        let mut system_message = prompts::get_base_system_prompt().to_string();

        if context_chunks.is_empty() {
            system_message.push_str(prompts::get_no_context_prompt());
        } else {
            system_message.push_str(prompts::get_context_header());

            for (i, chunk) in context_chunks.iter().enumerate() {
                system_message.push_str(&format!(
                    "---\n文档 {} (文件名: {}，相关度: {:.2})\n{}\n\n",
                    i + 1,
                    chunk.filename,
                    chunk.relevance_score,
                    chunk.content
                ));
            }

            system_message.push_str(prompts::get_context_footer());
        }

        system_message
    }

    async fn test_openai_connection(&self) -> Result<bool> {
        let url = format!("{}/models", self.config.base_url);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    async fn test_anthropic_connection(&self) -> Result<bool> {
        // Placeholder for Anthropic connection test
        Ok(false)
    }

    async fn test_local_connection(&self) -> Result<bool> {
        let response = self.client
            .get(&self.config.base_url)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    fn validate_config(config: &LlmConfig) -> Result<()> {
        if config.model.is_empty() {
            return Err(anyhow!("Model name cannot be empty"));
        }

        if config.base_url.is_empty() {
            return Err(anyhow!("Base URL cannot be empty"));
        }

        match config.provider {
            LlmProvider::OpenAI | LlmProvider::Anthropic => {
                if config.api_key.is_empty() {
                    return Err(anyhow!("API key is required for cloud providers"));
                }
            }
            LlmProvider::Local => {
                // API key is optional for local providers
            }
        }

        if let Some(temp) = config.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(anyhow!("Temperature must be between 0.0 and 2.0"));
            }
        }

        if let Some(max_tokens) = config.max_tokens {
            if max_tokens == 0 || max_tokens > 32000 {
                return Err(anyhow!("Max tokens must be between 1 and 32000"));
            }
        }

        Ok(())
    }

    pub fn update_config(&mut self, config: LlmConfig) -> Result<()> {
        Self::validate_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn get_config(&self) -> &LlmConfig {
        &self.config
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            api_key: String::new(),
            model: "gpt-4".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_tokens: Some(2000),
            temperature: Some(0.7),
            stream: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_validation() {
        let mut config = LlmConfig::default();
        config.api_key = "test_key".to_string();

        assert!(LlmClient::validate_config(&config).is_ok());

        // Test empty model
        config.model = String::new();
        assert!(LlmClient::validate_config(&config).is_err());

        // Test invalid temperature
        config.model = "gpt-4".to_string();
        config.temperature = Some(3.0);
        assert!(LlmClient::validate_config(&config).is_err());

        // Test invalid max_tokens
        config.temperature = Some(0.7);
        config.max_tokens = Some(0);
        assert!(LlmClient::validate_config(&config).is_err());
    }

    #[test]
    fn test_llm_provider_display() {
        assert_eq!(LlmProvider::OpenAI.to_string(), "OpenAI");
        assert_eq!(LlmProvider::Anthropic.to_string(), "Anthropic");
        assert_eq!(LlmProvider::Local.to_string(), "Local");
    }

    #[test]
    fn test_system_message_building() {
        let config = LlmConfig::default();
        let client = LlmClient::new(config).unwrap();

        // Test with no context
        let message = client.build_system_message(&[]);
        assert!(message.contains("MindKB"));
        assert!(message.contains("没有找到相关文档") || message.contains("当前查询"));

        // Test with context
        let context_chunks = vec![
            ContextChunk {
                document_id: "doc1".to_string(),
                filename: "test.txt".to_string(),
                content: "This is test content".to_string(),
                relevance_score: 0.9,
            }
        ];

        let message = client.build_system_message(&context_chunks);
        assert!(message.contains("MindKB"));
        assert!(message.contains("文档 1"));
        assert!(message.contains("test.txt"));
        assert!(message.contains("This is test content"));
    }

    #[test]
    fn test_chat_message_serialization() {
        let message = ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[tokio::test]
    async fn test_llm_client_creation() {
        let config = LlmConfig {
            provider: LlmProvider::OpenAI,
            api_key: "test_key".to_string(),
            model: "gpt-4".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: true,
        };

        let client = LlmClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_config_update() {
        let mut config = LlmConfig::default();
        config.api_key = "test_key".to_string();

        let mut client = LlmClient::new(config).unwrap();

        let new_config = LlmConfig {
            provider: LlmProvider::Local,
            api_key: String::new(),
            model: "local-model".to_string(),
            base_url: "http://localhost:8080".to_string(),
            max_tokens: Some(500),
            temperature: Some(0.5),
            stream: false,
        };

        assert!(client.update_config(new_config).is_ok());
        assert_eq!(client.get_config().provider, LlmProvider::Local);
        assert_eq!(client.get_config().model, "local-model");
        assert_eq!(client.get_config().stream, false);
    }
}
