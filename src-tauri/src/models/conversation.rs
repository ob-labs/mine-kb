use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "User"),
            MessageRole::Assistant => write!(f, "Assistant"),
            MessageRole::System => write!(f, "System"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: u32,
}

impl Conversation {
    pub fn new(project_id: Uuid, title: Option<String>) -> Result<Self, ConversationValidationError> {
        let title = title.unwrap_or_else(|| Self::generate_default_title());
        Self::validate_title(&title)?;

        let now = Utc::now();
        Ok(Conversation {
            id: Uuid::new_v4(),
            project_id,
            title,
            created_at: now,
            updated_at: now,
            message_count: 0,
        })
    }

    pub fn update_title(&mut self, title: String) -> Result<(), ConversationValidationError> {
        Self::validate_title(&title)?;
        self.title = title;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
        self.updated_at = Utc::now();
    }

    pub fn update_message_count(&mut self, count: u32) {
        self.message_count = count;
        self.updated_at = Utc::now();
    }

    fn validate_title(title: &str) -> Result<(), ConversationValidationError> {
        if title.trim().is_empty() {
            return Err(ConversationValidationError::EmptyTitle);
        }
        if title.len() > 200 {
            return Err(ConversationValidationError::TitleTooLong);
        }
        Ok(())
    }

    fn generate_default_title() -> String {
        let now = Utc::now();
        format!("{}", now.format("%Y-%m-%d %H:%M:%S"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: u32,
    pub context_chunks: Vec<Uuid>, // References to DocumentChunk IDs
    pub processing_time: Option<f64>, // Time taken to generate response (for Assistant messages)
    pub sources: Option<Vec<ContextChunk>>, // Source documents with filename and relevance
}

impl Message {
    pub fn new(
        conversation_id: Uuid,
        role: MessageRole,
        content: String,
    ) -> Result<Self, ConversationValidationError> {
        Self::validate_content(&content, &role)?;

        let token_count = Self::estimate_token_count(&content);

        Ok(Message {
            id: Uuid::new_v4(),
            conversation_id,
            role,
            content,
            timestamp: Utc::now(),
            token_count,
            context_chunks: Vec::new(),
            processing_time: None,
            sources: None,
        })
    }

    pub fn new_user_message(
        conversation_id: Uuid,
        content: String,
    ) -> Result<Self, ConversationValidationError> {
        Self::new(conversation_id, MessageRole::User, content)
    }

    pub fn new_assistant_message(
        conversation_id: Uuid,
        content: String,
        context_chunks: Vec<Uuid>,
        processing_time: Option<f64>,
    ) -> Result<Self, ConversationValidationError> {
        let mut message = Self::new(conversation_id, MessageRole::Assistant, content)?;
        message.context_chunks = context_chunks;
        message.processing_time = processing_time;
        Ok(message)
    }

    pub fn set_sources(&mut self, sources: Vec<ContextChunk>) {
        self.sources = Some(sources);
    }

    pub fn new_system_message(
        conversation_id: Uuid,
        content: String,
    ) -> Result<Self, ConversationValidationError> {
        Self::new(conversation_id, MessageRole::System, content)
    }

    pub fn add_context_chunk(&mut self, chunk_id: Uuid) {
        if !self.context_chunks.contains(&chunk_id) {
            self.context_chunks.push(chunk_id);
        }
    }

    pub fn set_processing_time(&mut self, time: f64) {
        self.processing_time = Some(time);
    }

    fn validate_content(content: &str, role: &MessageRole) -> Result<(), ConversationValidationError> {
        match role {
            MessageRole::User | MessageRole::Assistant => {
                if content.trim().is_empty() {
                    return Err(ConversationValidationError::EmptyMessageContent);
                }
            }
            MessageRole::System => {
                // System messages can be empty for certain use cases
            }
        }

        if content.len() > 10000 {
            return Err(ConversationValidationError::MessageTooLong);
        }

        Ok(())
    }

    fn estimate_token_count(content: &str) -> u32 {
        // Simple token estimation: roughly 4 characters per token
        (content.len() as f32 / 4.0).ceil() as u32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: u32,
}

impl From<Conversation> for ConversationResponse {
    fn from(conversation: Conversation) -> Self {
        ConversationResponse {
            id: conversation.id.to_string(),
            project_id: conversation.project_id.to_string(),
            title: conversation.title,
            created_at: conversation.created_at.to_rfc3339(),
            updated_at: conversation.updated_at.to_rfc3339(),
            message_count: conversation.message_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub token_count: u32,
    pub context_chunks: Vec<String>, // Chunk IDs as strings
    pub processing_time: Option<f64>,
}

impl From<Message> for MessageResponse {
    fn from(message: Message) -> Self {
        MessageResponse {
            id: message.id.to_string(),
            role: message.role.to_string(),
            content: message.content,
            timestamp: message.timestamp.to_rfc3339(),
            token_count: message.token_count,
            context_chunks: message.context_chunks.iter().map(|id| id.to_string()).collect(),
            processing_time: message.processing_time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub document_id: String,
    pub filename: String,
    pub content: String,
    pub relevance_score: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum ConversationValidationError {
    #[error("Conversation title cannot be empty")]
    EmptyTitle,
    #[error("Conversation title cannot exceed 200 characters")]
    TitleTooLong,
    #[error("Message content cannot be empty")]
    EmptyMessageContent,
    #[error("Message content cannot exceed 10000 characters")]
    MessageTooLong,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let project_id = Uuid::new_v4();
        let conversation = Conversation::new(project_id, Some("Test Conversation".to_string()));

        assert!(conversation.is_ok());
        let conversation = conversation.unwrap();
        assert_eq!(conversation.project_id, project_id);
        assert_eq!(conversation.title, "Test Conversation");
        assert_eq!(conversation.message_count, 0);
    }

    #[test]
    fn test_conversation_default_title() {
        let project_id = Uuid::new_v4();
        let conversation = Conversation::new(project_id, None).unwrap();

        assert!(conversation.title.starts_with("Conversation"));
        assert!(conversation.title.len() > 12); // Should include timestamp
    }

    #[test]
    fn test_conversation_validation() {
        let project_id = Uuid::new_v4();

        // Test empty title
        let result = Conversation::new(project_id, Some("".to_string()));
        assert!(result.is_err());

        // Test title too long
        let result = Conversation::new(project_id, Some("a".repeat(201)));
        assert!(result.is_err());
    }

    #[test]
    fn test_message_creation() {
        let conversation_id = Uuid::new_v4();

        // Test user message
        let message = Message::new_user_message(conversation_id, "Hello, AI!".to_string());
        assert!(message.is_ok());
        let message = message.unwrap();
        assert_eq!(message.conversation_id, conversation_id);
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, "Hello, AI!");
        assert!(message.token_count > 0);

        // Test assistant message
        let context_chunks = vec![Uuid::new_v4()];
        let message = Message::new_assistant_message(
            conversation_id,
            "Hello, human!".to_string(),
            context_chunks.clone(),
            Some(1.5),
        );
        assert!(message.is_ok());
        let message = message.unwrap();
        assert_eq!(message.role, MessageRole::Assistant);
        assert_eq!(message.context_chunks, context_chunks);
        assert_eq!(message.processing_time, Some(1.5));
    }

    #[test]
    fn test_message_validation() {
        let conversation_id = Uuid::new_v4();

        // Test empty user message
        let result = Message::new_user_message(conversation_id, "".to_string());
        assert!(result.is_err());

        // Test message too long
        let result = Message::new_user_message(conversation_id, "a".repeat(10001));
        assert!(result.is_err());

        // Test system message can be empty
        let result = Message::new_system_message(conversation_id, "".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_conversation_message_count() {
        let project_id = Uuid::new_v4();
        let mut conversation = Conversation::new(project_id, None).unwrap();

        assert_eq!(conversation.message_count, 0);

        conversation.increment_message_count();
        assert_eq!(conversation.message_count, 1);

        conversation.update_message_count(5);
        assert_eq!(conversation.message_count, 5);
    }

    #[test]
    fn test_message_context_chunks() {
        let conversation_id = Uuid::new_v4();
        let mut message = Message::new_user_message(conversation_id, "Test".to_string()).unwrap();

        let chunk_id = Uuid::new_v4();
        message.add_context_chunk(chunk_id);
        assert_eq!(message.context_chunks.len(), 1);
        assert_eq!(message.context_chunks[0], chunk_id);

        // Adding same chunk again should not duplicate
        message.add_context_chunk(chunk_id);
        assert_eq!(message.context_chunks.len(), 1);
    }

    #[test]
    fn test_response_conversion() {
        let project_id = Uuid::new_v4();
        let conversation = Conversation::new(project_id, Some("Test".to_string())).unwrap();
        let response: ConversationResponse = conversation.into();

        assert_eq!(response.title, "Test");
        assert!(response.id.len() > 0);

        let conversation_id = Uuid::new_v4();
        let message = Message::new_user_message(conversation_id, "Hello".to_string()).unwrap();
        let response: MessageResponse = message.into();

        assert_eq!(response.content, "Hello");
        assert_eq!(response.role, "User");
    }
}
