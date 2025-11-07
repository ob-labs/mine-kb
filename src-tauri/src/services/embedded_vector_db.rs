use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 嵌入式向量数据库，基于SQLite实现
#[derive(Debug)]
pub struct EmbeddedVectorDb {
    conn: Connection,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: VectorDocument,
    pub similarity: f64,
}

impl EmbeddedVectorDb {
    /// 创建新的嵌入式向量数据库实例
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path_str = db_path.as_ref().display().to_string();
        log::info!("🔗 [NEW-DB] 打开数据库文件: {}", db_path_str);

        // 获取数据库文件的绝对路径
        let absolute_path = std::fs::canonicalize(db_path.as_ref())
            .unwrap_or_else(|_| db_path.as_ref().to_path_buf());
        log::info!("🔗 [NEW-DB] 数据库绝对路径: {:?}", absolute_path);

        let conn = Connection::open(db_path)?;

        // 验证打开的是哪个数据库
        let db_file: String = conn.query_row(
            "PRAGMA database_list",
            [],
            |row| row.get(2)
        )?;
        log::info!("🔗 [NEW-DB] 实际连接的数据库: {}", db_file);

        // 启用外键约束并设置 WAL 模式和同步选项
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;"
        )?;

        log::info!("🔗 [NEW-DB] 数据库配置: foreign_keys=ON, journal_mode=WAL, synchronous=FULL");

        let mut db = Self { conn };
        db.initialize_schema()?;

        // 初始化后立即验证
        let msg_count: i32 = db.conn.query_row(
            "SELECT COUNT(*) FROM messages",
            [],
            |row| row.get(0)
        )?;
        let conv_count: i32 = db.conn.query_row(
            "SELECT COUNT(*) FROM conversations",
            [],
            |row| row.get(0)
        )?;
        log::info!("🔗 [NEW-DB] 数据库初始化完成，conversations: {}, messages: {}",
            conv_count, msg_count);

        Ok(db)
    }

    /// 创建内存数据库（用于测试）
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// 初始化数据库模式
    fn initialize_schema(&mut self) -> Result<()> {
        // 创建 projects 表
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                document_count INTEGER DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )",
            [],
        )?;

        // 创建 vector_documents 表
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS vector_documents (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                document_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB NOT NULL,
                metadata TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(document_id, chunk_index)
            )",
            [],
        )?;

        // 创建索引以提高查询性能
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_id ON vector_documents(project_id)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_document_id ON vector_documents(document_id)",
            [],
        )?;

        // 创建 conversations 表
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                message_count INTEGER DEFAULT 0,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 创建 messages 表
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                sources TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 如果 messages 表已存在但没有 sources 列，则添加（向后兼容）
        let has_sources_column = self.conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('messages') WHERE name='sources'")?
            .query_row([], |row| row.get::<_, i64>(0))
            .unwrap_or(0) > 0;

        if !has_sources_column {
            log::info!("添加 sources 列到 messages 表");
            self.conn.execute("ALTER TABLE messages ADD COLUMN sources TEXT", [])?;
        }

        // 创建对话表索引
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_conversation_project_id ON conversations(project_id)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_message_conversation_id ON messages(conversation_id)",
            [],
        )?;

        Ok(())
    }

    /// 添加向量文档
    pub fn add_document(&mut self, doc: VectorDocument) -> Result<()> {
        let embedding_bytes = bincode::serialize(&doc.embedding)?;
        let metadata_json = serde_json::to_string(&doc.metadata)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO vector_documents
             (id, project_id, document_id, chunk_index, content, embedding, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                doc.id,
                doc.project_id,
                doc.document_id,
                doc.chunk_index,
                doc.content,
                embedding_bytes,
                metadata_json
            ],
        )?;

        Ok(())
    }

    /// 批量添加向量文档
    pub fn add_documents(&mut self, docs: Vec<VectorDocument>) -> Result<()> {
        let tx = self.conn.transaction()?;

        for doc in docs {
            let embedding_bytes = bincode::serialize(&doc.embedding)?;
            let metadata_json = serde_json::to_string(&doc.metadata)?;

            tx.execute(
                "INSERT OR REPLACE INTO vector_documents
                 (id, project_id, document_id, chunk_index, content, embedding, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    doc.id,
                    doc.project_id,
                    doc.document_id,
                    doc.chunk_index,
                    doc.content,
                    embedding_bytes,
                    metadata_json
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// 向量相似度搜索
    pub fn similarity_search(
        &self,
        query_embedding: &[f64],
        project_id: Option<&str>,
        limit: usize,
        threshold: f64,
    ) -> Result<Vec<SearchResult>> {
        let mut query = "SELECT * FROM vector_documents".to_string();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(pid) = project_id {
            query.push_str(" WHERE project_id = ?");
            params.push(Box::new(pid.to_string()));
        }

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| self.row_to_vector_document(row),
        )?;

        let mut results = Vec::new();
        for row_result in rows {
            let doc = row_result?;
            let similarity = self.cosine_similarity(query_embedding, &doc.embedding);

            if similarity >= threshold {
                results.push(SearchResult {
                    document: doc,
                    similarity,
                });
            }
        }

        // 按相似度降序排序
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        // 限制结果数量
        results.truncate(limit);

        Ok(results)
    }

    /// 获取项目的所有文档
    pub fn get_project_documents(&self, project_id: &str) -> Result<Vec<VectorDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM vector_documents WHERE project_id = ? ORDER BY document_id, chunk_index"
        )?;

        let rows = stmt.query_map([project_id], |row| self.row_to_vector_document(row))?;

        let mut documents = Vec::new();
        for row_result in rows {
            documents.push(row_result?);
        }

        Ok(documents)
    }

    /// 删除项目的所有文档
    pub fn delete_project_documents(&mut self, project_id: &str) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM vector_documents WHERE project_id = ?",
            [project_id],
        )?;
        Ok(count)
    }

    /// 删除特定文档
    pub fn delete_document(&mut self, document_id: &str) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM vector_documents WHERE document_id = ?",
            [document_id],
        )?;
        Ok(count)
    }

    /// 获取数据库统计信息
    pub fn get_stats(&self) -> Result<HashMap<String, i64>> {
        let mut stats = HashMap::new();

        // 总文档数
        let total_docs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM vector_documents",
            [],
            |row| row.get(0),
        )?;
        stats.insert("total_documents".to_string(), total_docs);

        // 项目数
        let total_projects: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT project_id) FROM vector_documents",
            [],
            |row| row.get(0),
        )?;
        stats.insert("total_projects".to_string(), total_projects);

        Ok(stats)
    }

    /// 统计项目的文档数量（基于不同的 document_id）
    pub fn count_project_documents(&self, project_id: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT document_id) FROM vector_documents WHERE project_id = ?",
            [project_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// 将数据库行转换为VectorDocument
    fn row_to_vector_document(&self, row: &Row) -> rusqlite::Result<VectorDocument> {
        let embedding_bytes: Vec<u8> = row.get("embedding")?;
        let embedding: Vec<f64> = bincode::deserialize(&embedding_bytes)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Blob,
                Box::new(e)
            ))?;

        let metadata_json: String = row.get("metadata")?;
        let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e)
            ))?;

        Ok(VectorDocument {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            document_id: row.get("document_id")?,
            chunk_index: row.get("chunk_index")?,
            content: row.get("content")?,
            embedding,
            metadata,
        })
    }

    /// 计算余弦相似度
    fn cosine_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    /// 保存项目到数据库
    pub fn save_project(&mut self, project: &crate::models::project::Project) -> Result<()> {
        log::info!("💾 [SAVE-PROJECT] 保存项目: id={}, name={}, document_count={}",
            project.id, project.name, project.document_count);

        // 使用事务确保数据一致性
        let tx = self.conn.transaction()?;

        // ⚠️ 关键修复：使用 INSERT ... ON CONFLICT DO UPDATE 而不是 INSERT OR REPLACE
        // INSERT OR REPLACE 会触发 DELETE，导致 CASCADE 删除所有关联的 conversations 和 messages
        let rows_affected = tx.execute(
            "INSERT INTO projects (id, name, description, status, document_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                status = excluded.status,
                document_count = excluded.document_count,
                updated_at = excluded.updated_at",
            params![
                project.id.to_string(),
                project.name,
                project.description,
                project.status.to_string(),
                project.document_count as i64,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339()
            ],
        )?;

        // 提交事务
        tx.commit()?;

        log::info!("💾 [SAVE-PROJECT-END] 项目保存成功，rows_affected={}", rows_affected);

        Ok(())
    }

    /// 从数据库加载所有项目
    pub fn load_all_projects(&self) -> Result<Vec<crate::models::project::Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, status, document_count, created_at, updated_at
             FROM projects ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            use chrono::DateTime;
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let status_str: String = row.get(3)?;
            let document_count: i64 = row.get(4)?;
            let created_at_str: String = row.get(5)?;
            let updated_at_str: String = row.get(6)?;

            let id = uuid::Uuid::parse_str(&id)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;

            let status = match status_str.as_str() {
                "Created" => crate::models::project::ProjectStatus::Created,
                "Processing" => crate::models::project::ProjectStatus::Processing,
                "Ready" => crate::models::project::ProjectStatus::Ready,
                "Error" => crate::models::project::ProjectStatus::Error,
                _ => crate::models::project::ProjectStatus::Created,
            };

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&chrono::Utc);

            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&chrono::Utc);

            Ok(crate::models::project::Project {
                id,
                name,
                description,
                status,
                document_count: document_count as u32,
                created_at,
                updated_at,
            })
        })?;

        let mut projects = Vec::new();
        for row_result in rows {
            projects.push(row_result?);
        }

        Ok(projects)
    }

    /// 从数据库删除项目
    pub fn delete_project_by_id(&mut self, project_id: &str) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM projects WHERE id = ?",
            [project_id],
        )?;
        Ok(count)
    }

    /// 更新项目的文档数量
    pub fn update_project_document_count(&mut self, project_id: &str, count: u32) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET document_count = ?, updated_at = ? WHERE id = ?",
            params![
                count as i64,
                chrono::Utc::now().to_rfc3339(),
                project_id
            ],
        )?;
        Ok(())
    }

    // ==================== 对话管理方法 ====================

    /// 保存对话到数据库
    pub fn save_conversation(&mut self, conversation: &crate::models::conversation::Conversation) -> Result<()> {
        log::info!("💾 [SAVE-CONV-START] 保存对话: id={}, message_count={}",
            conversation.id, conversation.message_count);

        // 使用事务确保数据一致性
        let tx = self.conn.transaction()?;

        // ⚠️ 关键修复：使用 INSERT ... ON CONFLICT DO UPDATE 而不是 INSERT OR REPLACE
        // INSERT OR REPLACE 会触发 DELETE，导致 CASCADE 删除所有关联的 messages
        let rows_affected = tx.execute(
            "INSERT INTO conversations (id, project_id, title, created_at, updated_at, message_count)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                updated_at = excluded.updated_at,
                message_count = excluded.message_count",
            params![
                conversation.id.to_string(),
                conversation.project_id.to_string(),
                conversation.title,
                conversation.created_at.to_rfc3339(),
                conversation.updated_at.to_rfc3339(),
                conversation.message_count as i64,
            ],
        )?;

        // 提交事务
        tx.commit()?;

        log::info!("💾 [SAVE-CONV-END] 对话保存成功，rows_affected={}", rows_affected);

        Ok(())
    }

    /// 从数据库加载指定项目的所有对话
    pub fn load_conversations_by_project(&self, project_id: &str) -> Result<Vec<crate::models::conversation::Conversation>> {
        use uuid::Uuid;
        use chrono::DateTime;

        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title, created_at, updated_at, message_count
             FROM conversations
             WHERE project_id = ?
             ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([project_id], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: String = row.get(1)?;
            let title: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;
            let message_count: i64 = row.get(5)?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            let project_id = Uuid::parse_str(&project_id_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&chrono::Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&chrono::Utc);

            Ok(crate::models::conversation::Conversation {
                id,
                project_id,
                title,
                created_at,
                updated_at,
                message_count: message_count as u32,
            })
        })?;

        let mut conversations = Vec::new();
        for row_result in rows {
            conversations.push(row_result?);
        }

        Ok(conversations)
    }

    /// 从数据库加载所有对话
    pub fn load_all_conversations(&self) -> Result<Vec<crate::models::conversation::Conversation>> {
        use uuid::Uuid;
        use chrono::DateTime;

        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title, created_at, updated_at, message_count
             FROM conversations
             ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: String = row.get(1)?;
            let title: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;
            let message_count: i64 = row.get(5)?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            let project_id = Uuid::parse_str(&project_id_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&chrono::Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&chrono::Utc);

            Ok(crate::models::conversation::Conversation {
                id,
                project_id,
                title,
                created_at,
                updated_at,
                message_count: message_count as u32,
            })
        })?;

        let mut conversations = Vec::new();
        for row_result in rows {
            conversations.push(row_result?);
        }

        Ok(conversations)
    }

    /// 删除对话
    pub fn delete_conversation_by_id(&mut self, conversation_id: &str) -> Result<usize> {
        // 由于有 ON DELETE CASCADE，删除对话会自动删除相关消息
        let count = self.conn.execute(
            "DELETE FROM conversations WHERE id = ?",
            [conversation_id],
        )?;
        Ok(count)
    }

    /// 删除单条消息
    pub fn delete_message_by_id(&mut self, message_id: &str) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM messages WHERE id = ?",
            [message_id],
        )?;
        Ok(count)
    }

    /// 删除对话的所有消息
    pub fn delete_messages_by_conversation(&mut self, conversation_id: &str) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?",
            [conversation_id],
        )?;
        log::info!("删除对话 {} 的所有消息，共 {} 条", conversation_id, count);
        Ok(count)
    }

    /// 保存消息到数据库
    pub fn save_message(&mut self, message: &crate::models::conversation::Message) -> Result<()> {
        log::info!(
            "📝 [SAVE-MSG-START] id={}, conversation_id={}, role={}, content_len={}",
            message.id,
            message.conversation_id,
            message.role.to_string(),
            message.content.len()
        );

        // 在开始前查询总数
        let total_before: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages",
            [],
            |row| row.get(0)
        )?;
        log::info!("📝 [SAVE-MSG-START] 当前数据库messages总数（插入前）: {}", total_before);

        // ⭐ 添加：检查PRAGMA设置
        let foreign_keys_enabled: i32 = self.conn.query_row(
            "PRAGMA foreign_keys",
            [],
            |row| row.get(0)
        )?;
        log::info!("💡 当前连接 foreign_keys = {}", foreign_keys_enabled);

        if foreign_keys_enabled == 0 {
            log::warn!("⚠️  外键约束未启用，尝试启用...");
            self.conn.execute("PRAGMA foreign_keys = ON", [])?;
        }

        // ⭐ 添加：验证conversation存在
        let conv_exists: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM conversations WHERE id = ?",
            [message.conversation_id.to_string()],
            |row| row.get(0)
        )?;
        log::info!("💡 对话存在性检查: conversation_id={}, exists={}",
            message.conversation_id, conv_exists);

        if conv_exists == 0 {
            return Err(anyhow::anyhow!("对话不存在: {}", message.conversation_id));
        }

        // 使用事务确保数据一致性
        let tx = self.conn.transaction()?;

        log::info!("💡 事务已开启");

        // 序列化 sources 为 JSON
        let sources_json = message.sources.as_ref()
            .map(|sources| serde_json::to_string(sources).ok())
            .flatten();

        let rows_affected = match tx.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at, sources)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                message.id.to_string(),
                message.conversation_id.to_string(),
                message.role.to_string(),
                message.content,
                message.timestamp.to_rfc3339(),
                sources_json,
            ],
        ) {
            Ok(n) => {
                log::info!("✅ INSERT 成功: rows={}", n);
                n
            }
            Err(e) => {
                log::error!("❌ INSERT 失败: {}, 尝试 UPDATE", e);
                // 如果插入失败（可能是主键冲突），尝试更新
                tx.execute(
                    "UPDATE messages SET role=?, content=?, created_at=?, sources=? WHERE id=?",
                    params![
                        message.role.to_string(),
                        message.content,
                        message.timestamp.to_rfc3339(),
                        sources_json,
                        message.id.to_string(),
                    ],
                )?
            }
        };

        // ⭐ 添加：事务提交前验证数据
        let count_before_commit: i32 = tx.query_row(
            "SELECT COUNT(*) FROM messages WHERE id = ?",
            [message.id.to_string()],
            |row| row.get(0)
        )?;
        log::info!("💡 提交前验证: message_id={}, count={}", message.id, count_before_commit);

        // 提交事务
        match tx.commit() {
            Ok(_) => {
                log::info!("✅ [SAVE-MSG] 事务提交成功: rows_affected={}", rows_affected);
            }
            Err(e) => {
                log::error!("❌ [SAVE-MSG] 事务提交失败: {}", e);
                return Err(anyhow::anyhow!("事务提交失败: {}", e));
            }
        }

        // 提交后立即验证数据
        let count_after_commit: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE id = ?",
            [message.id.to_string()],
            |row| row.get(0)
        )?;
        log::info!("💡 [SAVE-MSG] 提交后验证: message_id={}, count={}", message.id, count_after_commit);

        // 再次确认连接的数据库文件
        let db_file: String = self.conn.query_row(
            "PRAGMA database_list",
            [],
            |row| row.get(2)
        )?;
        log::info!("💡 [SAVE-MSG] 当前操作的数据库文件: {}", db_file);

        // 检查所有消息总数
        let total_after: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages",
            [],
            |row| row.get(0)
        )?;
        log::info!("📝 [SAVE-MSG-END] 数据库messages总数（插入后）: {} -> {}",
            total_before, total_after);

        if total_after != total_before + 1 {
            log::warn!("⚠️  [SAVE-MSG] 警告：总数变化不正常！expected={}, actual={}",
                total_before + 1, total_after);
        }

        if count_after_commit == 0 {
            log::error!("🚨 [SAVE-MSG] 严重错误：事务提交成功但数据不在数据库中！");
            return Err(anyhow::anyhow!("数据未能持久化"));
        }

        log::info!("🎉 [SAVE-MSG-SUCCESS] message_id={}, 数据已确认写入", message.id);

        Ok(())
    }

    /// 获取消息总数（用于调试）
    pub fn get_message_count(&self) -> Result<i32> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages",
            [],
            |row| row.get(0)
        )?;
        Ok(count)
    }

    /// 获取特定对话的消息数量
    pub fn get_conversation_message_count(&self, conversation_id: &str) -> Result<i32> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ?",
            [conversation_id],
            |row| row.get(0)
        )?;
        Ok(count)
    }

    /// 从数据库加载对话的所有消息
    pub fn load_messages_by_conversation(&self, conversation_id: &str) -> Result<Vec<crate::models::conversation::Message>> {
        use uuid::Uuid;
        use chrono::DateTime;

        log::info!("load_messages_by_conversation: conversation_id={}", conversation_id);

        let mut stmt = self.conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, sources
             FROM messages
             WHERE conversation_id = ?
             ORDER BY created_at ASC"
        )?;

        let rows = stmt.query_map([conversation_id], |row| {
            let id_str: String = row.get(0)?;
            let conversation_id_str: String = row.get(1)?;
            let role_str: String = row.get(2)?;
            let content: String = row.get(3)?;
            let created_at_str: String = row.get(4)?;
            let sources_json: Option<String> = row.get(5)?;

            log::debug!("加载消息: id={}, role={}", id_str, role_str);

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            let conversation_id = Uuid::parse_str(&conversation_id_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&chrono::Utc);

            let role = match role_str.as_str() {
                // 匹配大写（数据库中的实际格式 - Display trait 输出）
                "User" => crate::models::conversation::MessageRole::User,
                "Assistant" => crate::models::conversation::MessageRole::Assistant,
                "System" => crate::models::conversation::MessageRole::System,
                // 兼容小写（向后兼容，可能存在的旧数据）
                "user" => crate::models::conversation::MessageRole::User,
                "assistant" => crate::models::conversation::MessageRole::Assistant,
                "system" => crate::models::conversation::MessageRole::System,
                _ => return Err(rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid role: '{}'", role_str)
                    ))
                ))
            };

            // 解析 sources JSON
            let sources = sources_json
                .and_then(|json| serde_json::from_str(&json).ok());

            Ok(crate::models::conversation::Message {
                id,
                conversation_id,
                role,
                content,
                timestamp: created_at,
                token_count: 0, // Not stored in DB, will be recalculated if needed
                context_chunks: Vec::new(), // Context not stored in DB
                processing_time: None, // Not stored in DB
                sources, // Load sources from DB
            })
        })?;

        let mut messages = Vec::new();
        for row_result in rows {
            match row_result {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    log::error!("解析消息行失败: {:?}", e);
                    return Err(anyhow::anyhow!("解析消息失败: {}", e));
                }
            }
        }

        log::info!("load_messages_by_conversation 完成: 加载了 {} 条消息", messages.len());
        Ok(messages)
    }
}

impl Drop for EmbeddedVectorDb {
    fn drop(&mut self) {
        log::warn!("🔥 [DB-DROP] 数据库连接即将关闭！");

        // 在关闭前检查数据
        if let Ok(msg_count) = self.conn.query_row::<i32, _, _>(
            "SELECT COUNT(*) FROM messages",
            [],
            |row| row.get(0)
        ) {
            log::warn!("🔥 [DB-DROP] 关闭时messages数量: {}", msg_count);
        }

        // 执行最终checkpoint
        if let Err(e) = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
            log::error!("🔥 [DB-DROP] 最终checkpoint失败: {}", e);
        } else {
            log::info!("🔥 [DB-DROP] 最终checkpoint完成");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_vector_db() -> Result<()> {
        let mut db = EmbeddedVectorDb::new_in_memory()?;

        let doc = VectorDocument {
            id: Uuid::new_v4().to_string(),
            project_id: Uuid::new_v4().to_string(),
            document_id: Uuid::new_v4().to_string(),
            chunk_index: 0,
            content: "测试文档内容".to_string(),
            embedding: vec![0.1, 0.2, 0.3, 0.4, 0.5],
            metadata: HashMap::new(),
        };

        db.add_document(doc.clone())?;

        let results = db.similarity_search(
            &[0.1, 0.2, 0.3, 0.4, 0.5],
            Some(&doc.project_id),
            10,
            0.0,
        )?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.content, "测试文档内容");
        assert!((results[0].similarity - 1.0).abs() < 0.001);

        Ok(())
    }
}
