// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

use mine_kb::commands::{chat, documents, projects, system, speech, initialization};
use mine_kb::services::app_state::AppState;
use mine_kb::config::AppConfig;
use mine_kb::app_state_wrapper::AppStateWrapper;
use mine_kb::AppDataDirPath;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, AppHandle};
use tokio::sync::Mutex;
use serde::Serialize;

/// 启动进度事件
#[derive(Debug, Clone, Serialize)]
struct StartupEvent {
    step: u32,
    total_steps: u32,
    message: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl StartupEvent {
    fn progress(step: u32, message: impl Into<String>) -> Self {
        Self {
            step,
            total_steps: 2,
            message: message.into(),
            status: "progress".to_string(),
            details: None,
            error: None,
        }
    }
    
    fn progress_with_details(step: u32, message: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            step,
            total_steps: 2,
            message: message.into(),
            status: "progress".to_string(),
            details: Some(details.into()),
            error: None,
        }
    }
    
    fn success(step: u32, message: impl Into<String>) -> Self {
        Self {
            step,
            total_steps: 2,
            message: message.into(),
            status: "success".to_string(),
            details: None,
            error: None,
        }
    }
    
    fn error(message: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            step: 0,
            total_steps: 2,
            message: message.into(),
            status: "error".to_string(),
            details: None,
            error: Some(error.into()),
        }
    }
}

/// 后台初始化任务
async fn initialize_app_async(
    app_handle: AppHandle,
    app_data_dir: PathBuf,
    db_path_str: String,
    model_cache_dir_str: Option<String>,
    state_wrapper: Arc<Mutex<Option<AppState>>>,
) {
    // 等待窗口显示
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // 发送初始事件
    let _ = app_handle.emit_all("startup-progress", StartupEvent::progress(0, "正在启动应用..."));
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  开始后台初始化");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // ============================================================
    // 1. 配置文件加载
    // ============================================================
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  步骤 1/2: 加载配置文件");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::progress(1, "加载配置文件"));
    
    let app_config = load_app_config(&app_data_dir);

    if app_config.is_none() {
        let example_config_path = app_data_dir.join("config.example.json");
        let example_config = AppConfig::default_config();
        if let Err(e) = example_config.save_to_file(&example_config_path) {
            log::error!("无法创建示例配置文件: {}", e);
        } else {
            log::info!("✅ 已创建示例配置文件: {:?}", example_config_path);
        }

        let error_msg = format!(
            "配置文件缺失\n\n请按照以下步骤配置：\n1. 打开文件夹: {}\n2. 编辑 config.example.json\n3. 将文件重命名为 config.json\n4. 重新启动应用",
            app_data_dir.display()
        );
        let _ = app_handle.emit_all("startup-progress", StartupEvent::error("配置文件缺失", error_msg));
        return;
    }
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::success(1, "配置文件加载完成"));

    // ============================================================
    // 2. 初始化应用状态
    // ============================================================
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  步骤 2/2: 初始化应用状态");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::progress_with_details(
        2,
        "初始化应用状态",
        "正在初始化向量数据库和AI服务..."
    ));
    
    log::info!("开始初始化应用状态...");
    
    let app_state_result = AppState::new_with_full_config(&db_path_str, app_config, model_cache_dir_str).await;

    match app_state_result {
        Ok(app_state) => {
            // 保存到状态包装器
            let mut state_guard = state_wrapper.lock().await;
            *state_guard = Some(app_state);
            
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::info!("  ✅ 应用启动成功！");
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            let _ = app_handle.emit_all("startup-progress", StartupEvent::success(2, "应用启动成功！"));
        }
        Err(e) => {
            log::error!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::error!("  ❌ 应用状态初始化失败");
            log::error!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            let _ = app_handle.emit_all("startup-progress", StartupEvent::error(
                "应用初始化失败",
                format!("{}", e)
            ));
        }
    }
}

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .setup(|app| {
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::info!("  Setup: 快速准备（非阻塞）");
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            // 应用数据目录：优先使用环境变量 CONFIG_DIR（本地开发可设），否则使用系统应用数据目录（与 Build/安装逻辑一致）
            let app_data_dir = std::env::var("CONFIG_DIR")
                .ok()
                .map(PathBuf::from)
                .or_else(|| app.path_resolver().app_data_dir())
                .expect("Failed to get app data directory (set CONFIG_DIR or use default)");
            if std::env::var("CONFIG_DIR").is_ok() {
                log::info!("使用 CONFIG_DIR 指定数据目录");
            }

            // 确保数据目录存在
            if !app_data_dir.exists() {
                fs::create_dir_all(&app_data_dir)
                    .expect("Failed to create app data directory");
            }
            // 规范为绝对路径，供前端 fs scope 校验通过（$APPDATA 解析为绝对路径）
            let app_data_dir = app_data_dir
                .canonicalize()
                .unwrap_or(app_data_dir);

            // 创建 tmp 目录（上传等临时文件），与数据目录一致
            let tmp_dir = app_data_dir.join("tmp");
            if !tmp_dir.exists() {
                fs::create_dir_all(&tmp_dir).expect("Failed to create tmp directory");
            }

            // 供前端获取（使 temp 等路径与后端一致；必须为绝对路径以匹配 tauri fs scope）
            let app_data_dir_str = app_data_dir
                .to_str()
                .expect("App data dir not UTF-8")
                .to_string();
            app.manage(AppDataDirPath(app_data_dir_str));

            // 嵌入模式数据目录：.../com.mine-kb.app/mine_kb.db/（SeekDB 实例目录，数据集中在此目录下不再平铺在 app_data_dir）
            let db_path = app_data_dir.join("mine_kb.db");
            if !db_path.exists() {
                fs::create_dir_all(&db_path).expect("Failed to create SeekDB data directory");
            }
            let db_path_str = db_path
                .to_str()
                .expect("Failed to convert database path to string")
                .to_string();

            log::info!("数据库目录: {}", db_path_str);

            // 创建模型缓存目录
            let model_cache_dir = app_data_dir.join("models");
            if !model_cache_dir.exists() {
                fs::create_dir_all(&model_cache_dir)
                    .expect("Failed to create model cache directory");
            }
            let model_cache_dir_str = model_cache_dir
                .to_str()
                .map(|s| s.to_string());

            log::info!("模型缓存目录: {:?}", model_cache_dir_str);

            // 首次运行检测：如果应用数据目录下没有config.json，尝试从resources目录复制
            let config_dest_path = app_data_dir.join("config.json");
            if !config_dest_path.exists() {
                log::info!("应用数据目录下未找到配置文件，尝试从resources目录复制...");

                // 获取resource目录路径
                if let Some(resource_dir) = app.path_resolver().resource_dir() {
                    let config_source_path = resource_dir.join("config.json");

                    if config_source_path.exists() {
                        match fs::copy(&config_source_path, &config_dest_path) {
                            Ok(_) => {
                                log::info!("✅ 已成功从resources目录复制配置文件到应用数据目录");
                                log::info!("   源: {:?}", config_source_path);
                                log::info!("   目标: {:?}", config_dest_path);
                            }
                            Err(e) => {
                                log::warn!("⚠️  无法复制配置文件: {}", e);
                            }
                        }
                    } else {
                        log::info!("resources目录下未找到config.json，将创建示例配置文件");
                    }
                }
            }

            // 创建状态包装器
            let state_wrapper = Arc::new(Mutex::new(None));
            let wrapper = AppStateWrapper {
                state: state_wrapper.clone(),
            };
            app.manage(wrapper);

            // 克隆 app_handle 用于后台任务
            let app_handle = app.handle();
            
            // 在后台异步初始化（不阻塞 setup）
            tauri::async_runtime::spawn(async move {
                initialize_app_async(
                    app_handle,
                    app_data_dir,
                    db_path_str,
                    model_cache_dir_str,
                    state_wrapper,
                ).await;
            });

            log::info!("✅ Setup 完成，窗口即将显示");
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Initialization commands
            initialization::trigger_initialization,
            initialization::check_initialization_status,
            // Project management commands
            projects::create_project,
            projects::get_projects,
            projects::get_project_details,
            projects::delete_project,
            projects::rename_project,
            // Document management commands
            documents::validate_files,
            documents::upload_documents,
            documents::get_document_content,
            // Chat/conversation commands
            chat::create_conversation,
            chat::send_message,
            chat::get_conversations,
            chat::get_conversation_history,
            chat::delete_conversation,
            chat::delete_message,
            chat::clear_messages,
            chat::rename_conversation,
            // System commands
            system::get_app_data_dir,
            system::save_file_to_app_tmp,
            system::get_app_status,
            system::configure_llm_service,
            system::select_directory,
            system::scan_directory,
            // Speech recognition commands
            speech::recognize_speech,
            speech::check_speech_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 开发时 src-tauri/config.json 的候选路径（tauri dev 时 cwd 可能是 target/debug，需多路径解析）
fn dev_config_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("src-tauri/config.json"),
        PathBuf::from("../src-tauri/config.json"),
        PathBuf::from("../../src-tauri/config.json"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
            candidates.push(root.join("src-tauri/config.json"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let from_cwd = cwd.join("src-tauri/config.json");
        if !candidates.contains(&from_cwd) {
            candidates.push(from_cwd);
        }
    }
    candidates
}

/// 加载应用配置
/// 开发时优先使用 src-tauri/config.json；打包运行后使用应用数据目录中的 config.json。
fn load_app_config(app_data_dir: &PathBuf) -> Option<AppConfig> {
    let mut config_paths = dev_config_candidates();
    config_paths.push(app_data_dir.join("config.json"));
    config_paths.push(PathBuf::from("config.json"));
    config_paths.push(PathBuf::from("../config.json"));

    for config_path in config_paths {
        if config_path.exists() {
            log::info!("尝试从配置文件读取: {:?}", config_path);
            match AppConfig::load_from_file(&config_path) {
                Ok(config) => {
                    let path_display = config_path.canonicalize().unwrap_or(config_path.clone());
                    log::info!("当前使用的配置文件（LLM API Key 由此文件提供）: {}", path_display.display());
                    log::info!("  - Model: {}", config.llm.model);
                    let key_preview = if config.llm.api_key.len() >= 12 {
                        format!("{}***", &config.llm.api_key[..12])
                    } else if config.llm.api_key.is_empty() {
                        "(空)".to_string()
                    } else {
                        "***".to_string()
                    };
                    log::info!("  - API Key: {} (长度 {}，若 401 请编辑上方路径对应文件中的 llm.apiKey)", key_preview, config.llm.api_key.len());
                    log::info!("  - Max Tokens: {:?}", config.llm.max_tokens);
                    log::info!("  - Temperature: {:?}", config.llm.temperature);
                    if let Some(base_url) = &config.llm.base_url {
                        if !base_url.is_empty() {
                            log::info!("  - LLM Base URL: {}", base_url);
                        }
                    }
                    if let Some(ref embedding_config) = config.embedding {
                        if let Some(ref emb_url) = embedding_config.base_url {
                            log::info!("  - Embedding Base URL: {}", emb_url);
                        }
                    }
                    return Some(config);
                }
                Err(e) => {
                    log::warn!("读取配置文件失败 {:?}: {}", config_path, e);
                }
            }
        }
    }

    log::info!("未找到配置文件，将尝试从环境变量读取");
    None
}
