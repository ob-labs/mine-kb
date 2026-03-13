use serde::{Deserialize, Serialize};
use tauri::command;
use crate::models::conversation::MessageRole;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub project_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub sources: Option<Vec<SourceResponse>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceResponse {
    pub filename: String,
    pub relevance_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteConversationRequest {
    pub conversation_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteMessageRequest {
    pub conversation_id: String,
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearMessagesRequest {
    pub conversation_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameConversationRequest {
    pub conversation_id: String,
    pub new_title: String,
}

#[command]
pub async fn create_conversation(
    request: CreateConversationRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<ConversationResponse, String> {
    log::info!("创建对话请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 project_id
    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|e| format!("无效的项目ID: {}", e))?;

    // 检查项目是否存在
    {
        let project_service = state.project_service();
        let project_service_guard = project_service.lock().await;
        if project_service_guard.get_project(project_id).is_none() {
            return Err(format!("项目不存在: {}", project_id));
        }
    }

    // 创建对话
    let conversation_id = {
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;

        conversation_service_guard
            .create_conversation(project_id, request.title)
            .await
            .map_err(|e| format!("创建对话失败: {}", e))?
    };

    // 获取创建的对话信息
    let conversation = {
        let conversation_service = state.conversation_service();
        let conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .get_conversation(conversation_id)
            .ok_or_else(|| "对话创建后未找到".to_string())?
            .clone()
    };

    let response = ConversationResponse {
        id: conversation.id.to_string(),
        project_id: conversation.project_id.to_string(),
        title: conversation.title,
        created_at: conversation.created_at.to_rfc3339(),
        updated_at: conversation.updated_at.to_rfc3339(),
        message_count: conversation.message_count,
    };

    log::info!("对话创建成功: {:?}", response);
    Ok(response)
}

#[command]
pub async fn get_conversations(
    project_id: String,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<Vec<ConversationResponse>, String> {
    log::info!("获取项目对话列表: {}", project_id);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 project_id
    let project_uuid = Uuid::parse_str(&project_id)
        .map_err(|e| format!("无效的项目ID: {}", e))?;

    // 获取对话列表
    let responses = {
        let conversation_service = state.conversation_service();
        let conversation_service_guard = conversation_service.lock().await;
        let conversations = conversation_service_guard.list_conversations(Some(project_uuid));

        // 立即转换为 owned 数据，避免生命周期问题
        conversations
            .iter()
            .map(|conv| ConversationResponse {
                id: conv.id.to_string(),
                project_id: conv.project_id.to_string(),
                title: conv.title.clone(),
                created_at: conv.created_at.to_rfc3339(),
                updated_at: conv.updated_at.to_rfc3339(),
                message_count: conv.message_count,
            })
            .collect::<Vec<ConversationResponse>>()
    };

    log::info!("找到 {} 个对话", responses.len());
    Ok(responses)
}

#[command]
pub async fn get_conversation_history(
    conversation_id: String,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<Vec<MessageResponse>, String> {
    log::info!("获取对话历史: {}", conversation_id);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 conversation_id
    let conversation_uuid = Uuid::parse_str(&conversation_id)
        .map_err(|e| format!("无效的对话ID: {}", e))?;

    // 获取消息列表
    let messages = {
        let conversation_service = state.conversation_service();
        let conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .get_conversation_messages(conversation_uuid)
            .map_err(|e| format!("获取对话历史失败: {}", e))?
    };

    let responses: Vec<MessageResponse> = messages
        .iter()
        .map(|msg| MessageResponse {
            id: msg.id.to_string(),
            conversation_id: msg.conversation_id.to_string(),
            role: msg.role.to_string().to_lowercase(),
            content: msg.content.clone(),
            created_at: msg.timestamp.to_rfc3339(),
            sources: msg.sources.as_ref().map(|sources| {
                sources.iter().map(|s| SourceResponse {
                    filename: s.filename.clone(),
                    relevance_score: s.relevance_score,
                }).collect()
            }),
        })
        .collect();

    log::info!("找到 {} 条消息", responses.len());
    Ok(responses)
}

#[command]
pub async fn send_message(
    request: SendMessageRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
    window: tauri::Window,
) -> Result<String, String> {
    log::info!("发送消息请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 conversation_id
    let conversation_uuid = Uuid::parse_str(&request.conversation_id)
        .map_err(|e| format!("无效的对话ID: {}", e))?;

    // 获取对话信息和项目ID
    let project_id = {
        let conversation_service = state.conversation_service();
        let conversation_service_guard = conversation_service.lock().await;
        let conversation = conversation_service_guard
            .get_conversation(conversation_uuid)
            .ok_or_else(|| "对话不存在".to_string())?;
        conversation.project_id
    };

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("💬 [CHAT] 开始处理对话消息");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("📋 对话ID: {}", conversation_uuid);
    log::info!("📁 项目ID: {}", project_id);
    log::info!("💬 用户消息: {}", request.content);
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 1. 保存用户消息
    log::info!("💾 [CHAT] 步骤 1/5: 保存用户消息到数据库");
    {
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .add_message(conversation_uuid, MessageRole::User, request.content.clone())
            .await
            .map_err(|e| format!("保存用户消息失败: {}", e))?;
    }
    log::info!("✅ [CHAT] 用户消息已保存");

    // 2. 向量检索：从知识库检索相关文档块（使用SeekDB向量搜索）
    log::info!("🔍 [CHAT] 步骤 2/5: 执行SeekDB向量检索");
    let context_chunks = {
        let document_service = state.document_service();
        let document_service_guard = document_service.lock().await;

        match document_service_guard.search_similar_chunks_for_chat(&project_id.to_string(), &request.content, 5).await {
            Ok(chunks) => {
                log::info!("✅ [CHAT] SeekDB向量检索成功，找到 {} 个相关文档块", chunks.len());
                
                // 打印每个文档块的详细信息
                for (i, chunk) in chunks.iter().enumerate() {
                    log::info!("   📄 上下文块 #{}: 文件={:?}, 相关度={:.4}", 
                        i + 1, 
                        chunk.filename.as_ref().unwrap_or(&"未知".to_string()),
                        chunk.relevance_score
                    );
                    log::info!("      内容: {}...", 
                        chunk.content.chars().take(100).collect::<String>()
                    );
                }
                
                chunks.into_iter().map(|chunk| {
                    crate::models::conversation::ContextChunk {
                        document_id: chunk.document_id,
                        filename: chunk.filename.unwrap_or_else(|| "未知文档".to_string()),
                        content: chunk.content,
                        relevance_score: chunk.relevance_score,
                    }
                }).collect::<Vec<_>>()
            }
            Err(e) => {
                log::warn!("⚠️  [CHAT] 混合检索失败: {}，将不使用上下文", e);
                Vec::new()
            }
        }
    };
    
    if context_chunks.is_empty() {
        log::warn!("⚠️  [CHAT] 没有找到相关文档，AI 将基于通用知识回答");
    } else {
        log::info!("✅ [CHAT] 将使用 {} 个文档块作为上下文", context_chunks.len());
    }

    // 3. 获取对话历史
    log::info!("📜 [CHAT] 步骤 3/5: 获取对话历史");
    let messages = {
        let conversation_service = state.conversation_service();
        let conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .get_conversation_messages(conversation_uuid)
            .map_err(|e| format!("获取对话历史失败: {}", e))?
    };
    log::info!("✅ [CHAT] 获取到 {} 条历史消息", messages.len());
    
    // 打印对话历史（最近3条）
    for (i, msg) in messages.iter().rev().take(3).rev().enumerate() {
        log::info!("   消息 #{}: {} - {}", 
            i + 1,
            msg.role.to_string(),
            msg.content.chars().take(50).collect::<String>()
        );
    }

    // 4. 调用 LLM 生成响应（流式）
    log::info!("🤖 [CHAT] 步骤 4/5: 调用 LLM 生成响应");
    log::info!("   上下文块数量: {}", context_chunks.len());
    log::info!("   历史消息数量: {}", messages.len());
    use futures::StreamExt;
    use crate::services::llm_client::StreamEvent;

    let mut response_content = String::new();

    {
        let llm_client = state.llm_client();
        let llm_client_guard = llm_client.lock().await;

        let mut stream = llm_client_guard
            .generate_response(&messages, &context_chunks)
            .await
            .map_err(|e| {
                log::error!("❌ [CHAT] LLM 调用失败: {}", e);
                format!("LLM 调用失败: {}", e)
            })?;
        
        log::info!("✅ [CHAT] LLM 流式响应已建立");

        // 发送流式开始事件
        let _ = window.emit("chat-stream-start", request.conversation_id.clone());

        // 发送来源文档信息
        if !context_chunks.is_empty() {
            let sources: Vec<serde_json::Value> = context_chunks.iter().map(|chunk| {
                serde_json::json!({
                    "filename": chunk.filename,
                    "relevance_score": chunk.relevance_score,
                })
            }).collect();

            let _ = window.emit("chat-stream-context", serde_json::json!({
                "conversation_id": request.conversation_id,
                "sources": sources
            }));
        }

        // 流式处理响应
        let mut token_count = 0;
        while let Some(event) = stream.next().await {
            match event {
                StreamEvent::Token(token) => {
                    response_content.push_str(&token);
                    token_count += 1;

                    // 立即发送 token 到前端
                    let _ = window.emit("chat-stream-token", serde_json::json!({
                        "conversation_id": request.conversation_id,
                        "token": token
                    }));
                }
                StreamEvent::Context(_) => {
                    log::debug!("   收到上下文信息");
                }
                StreamEvent::Complete(response_id) => {
                    log::info!("✅ [CHAT] LLM 响应完成: {}", response_id);
                    log::info!("   总 token 数: {}", token_count);
                    log::info!("   响应长度: {} 字符", response_content.len());
                }
                StreamEvent::Error(error) => {
                    log::error!("❌ [CHAT] 流式响应错误: {}", error);
                    let _ = window.emit("chat-stream-error", serde_json::json!({
                        "conversation_id": request.conversation_id,
                        "error": error.clone()
                    }));
                    return Err(format!("LLM 响应错误: {}", error));
                }
            }
        }
        
        log::info!("🎉 [CHAT] 流式传输完成，共收到 {} 个 token", token_count);
    }

    if response_content.is_empty() {
        log::error!("❌ [CHAT] LLM 未返回有效响应");
        return Err("LLM 未返回有效响应".to_string());
    }
    
    log::info!("📝 [CHAT] AI 响应内容预览: {}...", 
        response_content.chars().take(100).collect::<String>()
    );

    // 5. 保存 AI 响应消息（包含 sources）
    log::info!("💾 [CHAT] 步骤 5/5: 保存 AI 响应到数据库");
    let message_id = {
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .add_message(conversation_uuid, MessageRole::Assistant, response_content.clone())
            .await
            .map_err(|e| {
                log::error!("❌ [CHAT] 保存 AI 消息失败: {}", e);
                format!("保存 AI 消息失败: {}", e)
            })?
    }; // 释放 conversation_service 锁
    
    log::info!("✅ [CHAT] AI 消息已保存，消息ID: {}", message_id);

    // 如果有 sources，更新消息并保存到数据库
    if !context_chunks.is_empty() {
        log::info!("📎 [CHAT] 附加来源文档信息（{} 个）", context_chunks.len());
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;

        if let Some(message) = conversation_service_guard.get_message_mut(conversation_uuid, message_id) {
            // 设置 sources
            message.set_sources(context_chunks.clone());

            // 保存到数据库
            let message_clone = message.clone();
            drop(conversation_service_guard); // 显式释放 conversation_service 锁

            let document_service = state.document_service();
            let doc_service_guard = document_service.lock().await;
            let db = doc_service_guard.get_vector_db();
            let adapter = db.lock().await.clone();
            adapter.save_message(&message_clone)
                .await
                .map_err(|e| {
                    log::error!("❌ [CHAT] 更新消息 sources 失败: {}", e);
                    format!("更新消息 sources 失败: {}", e)
                })?;
            
            log::info!("✅ [CHAT] 来源文档信息已附加");
        }
    } else {
        log::info!("ℹ️  [CHAT] 没有来源文档信息需要附加");
    }

    // 在所有保存操作完成后，才发送流式结束事件
    let _ = window.emit("chat-stream-end", serde_json::json!({
        "conversation_id": request.conversation_id,
        "content": response_content.clone()
    }));

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🎉 [CHAT] 对话处理完成！");
    log::info!("   对话ID: {}", conversation_uuid);
    log::info!("   响应长度: {} 字符", response_content.len());
    log::info!("   使用了 {} 个上下文文档块", context_chunks.len());
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(response_content)
}

#[command]
pub async fn delete_conversation(
    request: DeleteConversationRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<bool, String> {
    log::info!("删除对话请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 conversation_id
    let conversation_uuid = Uuid::parse_str(&request.conversation_id)
        .map_err(|e| format!("无效的对话ID: {}", e))?;

    // 删除对话
    {
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .delete_conversation(conversation_uuid)
            .await
            .map_err(|e| format!("删除对话失败: {}", e))?;
    }

    log::info!("对话删除成功: {}", conversation_uuid);
    Ok(true)
}

#[command]
pub async fn delete_message(
    request: DeleteMessageRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<bool, String> {
    log::info!("删除消息请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 conversation_id 和 message_id
    let conversation_uuid = Uuid::parse_str(&request.conversation_id)
        .map_err(|e| format!("无效的对话ID: {}", e))?;
    let message_uuid = Uuid::parse_str(&request.message_id)
        .map_err(|e| format!("无效的消息ID: {}", e))?;

    // 删除消息
    {
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .delete_message(conversation_uuid, message_uuid)
            .await
            .map_err(|e| format!("删除消息失败: {}", e))?;
    }

    log::info!("消息删除成功: {}", message_uuid);
    Ok(true)
}

#[command]
pub async fn clear_messages(
    request: ClearMessagesRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<bool, String> {
    log::info!("清空消息请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 conversation_id
    let conversation_uuid = Uuid::parse_str(&request.conversation_id)
        .map_err(|e| format!("无效的对话ID: {}", e))?;

    // 清空对话的所有消息
    {
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .clear_conversation_messages(conversation_uuid)
            .await
            .map_err(|e| format!("清空消息失败: {}", e))?;
    }

    log::info!("消息清空成功: {}", conversation_uuid);
    Ok(true)
}

#[command]
pub async fn rename_conversation(
    request: RenameConversationRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<bool, String> {
    log::info!("重命名对话请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证 conversation_id
    let conversation_uuid = Uuid::parse_str(&request.conversation_id)
        .map_err(|e| format!("无效的对话ID: {}", e))?;

    // 验证新标题不为空
    let trimmed_title = request.new_title.trim();
    if trimmed_title.is_empty() {
        return Err("对话标题不能为空".to_string());
    }

    // 重命名对话
    {
        let conversation_service = state.conversation_service();
        let mut conversation_service_guard = conversation_service.lock().await;
        conversation_service_guard
            .update_conversation_title(conversation_uuid, trimmed_title.to_string())
            .await
            .map_err(|e| format!("重命名对话失败: {}", e))?;
    }

    log::info!("对话重命名成功: {}", conversation_uuid);
    Ok(true)
}
