use crate::services::app_state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 应用状态包装器，支持延迟初始化
pub struct AppStateWrapper {
    pub state: Arc<Mutex<Option<AppState>>>,
}

impl AppStateWrapper {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
        }
    }
    
    /// 获取已初始化的 AppState，如果未初始化则返回错误
    pub async fn get_state(&self) -> Result<AppState, String> {
        let state_guard = self.state.lock().await;
        match state_guard.as_ref() {
            Some(state) => Ok(AppState {
                project_service: state.project_service.clone(),
                document_service: state.document_service.clone(),
                conversation_service: state.conversation_service.clone(),
                llm_client: state.llm_client.clone(),
            }),
            None => Err("应用正在初始化，请稍候...".to_string()),
        }
    }
}

