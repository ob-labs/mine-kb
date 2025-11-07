use tauri::{command, AppHandle, State};
use crate::app_state_wrapper::AppStateWrapper;

/// 前端调用此命令以触发应用初始化
/// 这样可以确保前端已经准备好接收启动事件
#[command]
pub async fn trigger_initialization(
    _app_handle: AppHandle,
    wrapper: State<'_, AppStateWrapper>,
) -> Result<(), String> {
    log::info!("前端触发初始化请求");
    
    // 检查是否已经初始化
    {
        let state_guard = wrapper.state.lock().await;
        if state_guard.is_some() {
            log::info!("应用已经初始化，跳过");
            return Ok(());
        }
    }
    
    log::info!("开始后台初始化...");
    
    // 这里的逻辑已经在 setup 中的异步任务里执行
    // 前端只需要等待事件即可
    
    Ok(())
}

/// 检查初始化状态
#[command]
pub async fn check_initialization_status(
    wrapper: State<'_, AppStateWrapper>,
) -> Result<bool, String> {
    let state_guard = wrapper.state.lock().await;
    Ok(state_guard.is_some())
}

