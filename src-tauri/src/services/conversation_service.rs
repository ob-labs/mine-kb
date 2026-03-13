use crate::models::conversation::{Conversation, Message, MessageRole};
use crate::services::seekdb_adapter::SeekDbAdapter;
use anyhow::{anyhow, Result};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ConversationService {
    conversations: HashMap<Uuid, Conversation>,
    messages: HashMap<Uuid, Vec<Message>>, // conversation_id -> messages
    db: Arc<Mutex<SeekDbAdapter>>,
}

impl ConversationService {
    pub async fn new(db: Arc<Mutex<SeekDbAdapter>>) -> Self {
        log::info!("ConversationService 初始化开始...");

        let mut service = Self {
            conversations: HashMap::new(),
            messages: HashMap::new(),
            db: db.clone(),
        };

        // 从数据库加载所有对话
        log::info!("准备从数据库加载对话和消息...");
        match service.load_from_database().await {
            Ok(_) => {
                log::info!("ConversationService 初始化完成: {} 个对话，{} 条消息",
                    service.conversations.len(),
                    service.messages.values().map(|v| v.len()).sum::<usize>()
                );
            }
            Err(e) => {
                log::error!("❌ 从数据库加载对话失败: {}", e);
                log::error!("错误详情: {:?}", e);
            }
        }

        service
    }

    /// 从数据库加载所有对话和消息
    async fn load_from_database(&mut self) -> Result<()> {
        log::info!("load_from_database: 开始执行");

        let adapter = self.db.lock().await.clone();
        log::info!("load_from_database: 成功获取数据库锁");

        let conversations = adapter.load_all_conversations().await?;
        log::info!("✅ 从数据库加载了 {} 个对话", conversations.len());

        for conv in conversations {
            let conv_id = conv.id;
            log::info!("处理对话: id={}, title={}", conv_id, conv.title);

            match adapter.load_messages_by_conversation(&conv_id.to_string()).await {
                Ok(messages) => {
                    log::info!("✅ 对话 {} 加载了 {} 条消息", conv_id, messages.len());
                    self.conversations.insert(conv_id, conv);
                    self.messages.insert(conv_id, messages);
                }
                Err(e) => {
                    log::error!("❌ 对话 {} 加载消息失败: {}", conv_id, e);
                    log::error!("错误详情: {:?}", e);
                    self.conversations.insert(conv_id, conv);
                    self.messages.insert(conv_id, Vec::new());
                }
            }
        }

        log::info!("load_from_database: 完成");
        Ok(())
    }

    pub async fn create_conversation(&mut self, project_id: Uuid, title: Option<String>) -> Result<Uuid> {
        let conversation = Conversation::new(project_id, title)?;
        let conversation_id = conversation.id;

        let adapter = self.db.lock().await.clone();
        adapter.save_conversation(&conversation).await?;

        self.conversations.insert(conversation_id, conversation);
        self.messages.insert(conversation_id, Vec::new());
        Ok(conversation_id)
    }

    pub fn get_conversation(&self, conversation_id: Uuid) -> Option<&Conversation> {
        self.conversations.get(&conversation_id)
    }

    pub fn get_conversation_mut(&mut self, conversation_id: Uuid) -> Option<&mut Conversation> {
        self.conversations.get_mut(&conversation_id)
    }

    pub fn list_conversations(&self, project_id: Option<Uuid>) -> Vec<&Conversation> {
        let mut conversations: Vec<&Conversation> = self.conversations
            .values()
            .filter(|conv| {
                if let Some(pid) = project_id {
                    conv.project_id == pid
                } else {
                    true
                }
            })
            .collect();

        // 按更新时间降序排序（最新的在前）
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        conversations
    }

    pub async fn add_message(&mut self, conversation_id: Uuid, role: MessageRole, content: String) -> Result<Uuid> {
        log::info!("add_message 开始: conversation_id={}, role={:?}", conversation_id, role);

        let conversation = self.conversations
            .get_mut(&conversation_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", conversation_id))?;

        let message = Message::new(conversation_id, role, content)?;
        let message_id = message.id;
        log::info!("创建消息对象成功: message_id={}", message_id);

        {
            let adapter = self.db.lock().await.clone();
            let count = adapter.get_message_count().await.unwrap_or(-1);
            log::warn!("🔍 [BEFORE-SAVE] 锁定数据库前，messages总数: {}", count);
        }

        {
            log::info!("尝试获取数据库锁以保存消息...");
            let adapter = self.db.lock().await.clone();
            log::info!("成功获取数据库锁");
            log::info!("调用 save_message...");
            adapter.save_message(&message).await?;
            log::info!("消息保存到数据库成功");

            let count = adapter.get_message_count().await.unwrap_or(-1);
            log::warn!("🔍 [AFTER-SAVE-IN-LOCK] 保存后，释放锁前，messages总数: {}", count);
        }

        // ⭐ 释放锁后立即检查
        {
            let adapter = self.db.lock().await.clone();
            let count = adapter.get_message_count().await.unwrap_or(-1);
            log::warn!("🔍 [AFTER-LOCK-RELEASE] 释放锁后，messages总数: {}", count);
        }

        // Add message to messages collection
        let messages = self.messages.entry(conversation_id).or_insert_with(Vec::new);
        messages.push(message);
        log::info!("消息添加到内存集合成功");

        // Update conversation
        conversation.increment_message_count();
        log::info!("对话消息计数已更新");

        {
            log::info!("尝试获取数据库锁以更新对话...");
            let adapter = self.db.lock().await.clone();
            log::info!("成功获取数据库锁");

            let count = adapter.get_message_count().await.unwrap_or(-1);
            log::warn!("🔍 [BEFORE-UPDATE-CONV] 更新对话前，messages总数: {}", count);

            log::info!("调用 save_conversation...");
            adapter.save_conversation(conversation).await?;
            log::info!("对话更新到数据库成功");

            let count = adapter.get_message_count().await.unwrap_or(-1);
            log::warn!("🔍 [AFTER-UPDATE-CONV] 更新对话后，messages总数: {}", count);
        }

        log::info!("add_message 完成: message_id={}", message_id);
        Ok(message_id)
    }

    pub async fn update_conversation_title(&mut self, conversation_id: Uuid, title: String) -> Result<()> {
        let conversation = self.conversations
            .get_mut(&conversation_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", conversation_id))?;

        conversation.update_title(title)?;

        {
            let adapter = self.db.lock().await.clone();
            adapter.save_conversation(conversation).await?;
        }

        Ok(())
    }

    pub async fn delete_conversation(&mut self, conversation_id: Uuid) -> Result<()> {
        {
            let adapter = self.db.lock().await.clone();
            adapter.delete_conversation_by_id(&conversation_id.to_string()).await?;
        }

        self.conversations
            .remove(&conversation_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", conversation_id))?;
        self.messages.remove(&conversation_id);
        Ok(())
    }

    pub async fn delete_message(&mut self, conversation_id: Uuid, message_id: Uuid) -> Result<()> {
        // 验证对话是否存在
        let conversation = self.conversations
            .get_mut(&conversation_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", conversation_id))?;

        // 从内存中删除消息
        let messages = self.messages.entry(conversation_id).or_insert_with(Vec::new);
        let original_len = messages.len();
        messages.retain(|msg| msg.id != message_id);

        if messages.len() == original_len {
            return Err(anyhow!("Message not found: {}", message_id));
        }

        {
            let adapter = self.db.lock().await.clone();
            adapter.delete_message_by_id(&message_id.to_string()).await?;
        }

        conversation.update_message_count(messages.len() as u32);

        {
            let adapter = self.db.lock().await.clone();
            adapter.save_conversation(conversation).await?;
        }

        Ok(())
    }

    pub async fn clear_conversation_messages(&mut self, conversation_id: Uuid) -> Result<()> {
        let conversation = self.conversations
            .get_mut(&conversation_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", conversation_id))?;

        {
            let adapter = self.db.lock().await.clone();
            adapter.delete_messages_by_conversation(&conversation_id.to_string()).await?;
        }

        self.messages.entry(conversation_id).or_insert_with(Vec::new).clear();
        conversation.update_message_count(0);

        {
            let adapter = self.db.lock().await.clone();
            adapter.save_conversation(conversation).await?;
        }

        Ok(())
    }

    pub fn get_conversation_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        log::info!("get_conversation_messages: conversation_id={}", conversation_id);

        self.conversations
            .get(&conversation_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", conversation_id))?;

        let mut messages = self.messages.get(&conversation_id).cloned().unwrap_or_default();
        
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then_with(|| a.id.cmp(&b.id)));
        
        log::info!("get_conversation_messages: 从内存返回 {} 条消息（已按时间排序）", messages.len());

        Ok(messages)
    }

    pub fn get_message_mut(&mut self, conversation_id: Uuid, message_id: Uuid) -> Option<&mut Message> {
        self.messages
            .get_mut(&conversation_id)?
            .iter_mut()
            .find(|msg| msg.id == message_id)
    }

    pub fn count_conversations(&self, project_id: Option<Uuid>) -> usize {
        if let Some(pid) = project_id {
            self.conversations
                .values()
                .filter(|conv| conv.project_id == pid)
                .count()
        } else {
            self.conversations.len()
        }
    }
}

// 注意：ConversationService 不再实现 Default，因为它需要数据库引用

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::seekdb_adapter::SeekDbAdapter;
    use std::sync::Arc;

    async fn test_service() -> ConversationService {
        let path = std::env::temp_dir().join(format!("mine_kb_test_conv_{}.db", std::process::id()));
        let adapter = SeekDbAdapter::new_async(&path).await.unwrap();
        ConversationService::new(Arc::new(Mutex::new(adapter))).await
    }

    #[tokio::test]
    async fn test_conversation_service_creation() {
        let service = test_service().await;
        assert_eq!(service.conversations.len(), 0);
    }

    #[tokio::test]
    async fn test_create_and_get_conversation() {
        let mut service = test_service().await;
        let project_id = Uuid::new_v4();

        let conversation_id = service.create_conversation(project_id, Some("Test Conversation".to_string())).await.unwrap();
        let conversation = service.get_conversation(conversation_id).unwrap();

        assert_eq!(conversation.title, "Test Conversation");
        assert_eq!(conversation.project_id, project_id);
        assert_eq!(service.get_conversation_messages(conversation_id).unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_add_message() {
        let mut service = test_service().await;
        let project_id = Uuid::new_v4();

        let conversation_id = service.create_conversation(project_id, Some("Test".to_string())).await.unwrap();
        let message_id = service.add_message(conversation_id, MessageRole::User, "Hello".to_string()).await.unwrap();

        let messages = service.get_conversation_messages(conversation_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, message_id);
        assert_eq!(messages[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_list_conversations_by_project() {
        let mut service = test_service().await;
        let project1 = Uuid::new_v4();
        let project2 = Uuid::new_v4();

        service.create_conversation(project1, Some("Conv 1".to_string())).await.unwrap();
        service.create_conversation(project1, Some("Conv 2".to_string())).await.unwrap();
        service.create_conversation(project2, Some("Conv 3".to_string())).await.unwrap();

        let project1_conversations = service.list_conversations(Some(project1));
        assert_eq!(project1_conversations.len(), 2);

        let all_conversations = service.list_conversations(None);
        assert_eq!(all_conversations.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_conversation() {
        let mut service = test_service().await;
        let project_id = Uuid::new_v4();

        let conversation_id = service.create_conversation(project_id, Some("Test".to_string())).await.unwrap();
        assert!(service.get_conversation(conversation_id).is_some());

        service.delete_conversation(conversation_id).await.unwrap();
        assert!(service.get_conversation(conversation_id).is_none());
    }
}
