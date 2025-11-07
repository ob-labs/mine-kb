// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

use mine_kb::commands::{chat, documents, projects, system, speech, initialization};
use mine_kb::services::app_state::AppState;
use mine_kb::services::python_env::PythonEnv;
use mine_kb::services::seekdb_package::SeekDbPackage;
use mine_kb::config::AppConfig;
use mine_kb::app_state_wrapper::AppStateWrapper;
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
            total_steps: 3,
            message: message.into(),
            status: "progress".to_string(),
            details: None,
            error: None,
        }
    }
    
    fn progress_with_details(step: u32, message: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            step,
            total_steps: 3,
            message: message.into(),
            status: "progress".to_string(),
            details: Some(details.into()),
            error: None,
        }
    }
    
    fn success(step: u32, message: impl Into<String>) -> Self {
        Self {
            step,
            total_steps: 3,
            message: message.into(),
            status: "success".to_string(),
            details: None,
            error: None,
        }
    }
    
    fn error(message: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            step: 0,
            total_steps: 3,
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
    // 1. Python 环境和 SeekDB 安装
    // ============================================================
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  步骤 1/3: 初始化 Python 环境和 SeekDB");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::progress(1, "初始化 Python 环境"));
    
    // 创建 Python 虚拟环境
    let python_env = match PythonEnv::new(&app_data_dir) {
        Ok(env) => env,
        Err(e) => {
            log::error!("Python 环境初始化失败: {}", e);
            let _ = app_handle.emit_all("startup-progress", StartupEvent::error(
                "Python 环境初始化失败",
                format!("{}", e)
            ));
            return;
        }
    };
    
    if let Err(e) = python_env.ensure_venv() {
        log::error!("Python 虚拟环境创建失败: {}", e);
        let _ = app_handle.emit_all("startup-progress", StartupEvent::error(
            "Python 虚拟环境创建失败",
            format!("{}", e)
        ));
        return;
    }
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::progress(1, "检查 SeekDB 包"));
    
    // 检查并安装 SeekDB
    let seekdb_pkg = SeekDbPackage::new(&python_env);
    
    match seekdb_pkg.is_installed() {
        Ok(false) => {
            log::info!("📦 SeekDB 未安装，开始安装...");
            let _ = app_handle.emit_all("startup-progress", StartupEvent::progress_with_details(
                1,
                "安装 SeekDB",
                "首次运行需要下载并安装 SeekDB（约3GB），可能需要几分钟..."
            ));
            
            if let Err(e) = seekdb_pkg.install() {
                log::error!("SeekDB 安装失败: {}", e);
                let _ = app_handle.emit_all("startup-progress", StartupEvent::error(
                    "SeekDB 安装失败",
                    format!("{}", e)
                ));
                return;
            }
        }
        Ok(true) => {
            log::info!("✅ SeekDB 已安装");
        }
        Err(e) => {
            log::warn!("⚠️  检查 SeekDB 安装状态失败，尝试安装: {}", e);
            if let Err(e) = seekdb_pkg.install() {
                log::error!("SeekDB 安装失败: {}", e);
                let _ = app_handle.emit_all("startup-progress", StartupEvent::error(
                    "SeekDB 安装失败",
                    format!("{}", e)
                ));
                return;
            }
        }
    }
    
    if let Err(e) = seekdb_pkg.verify() {
        log::error!("SeekDB 验证失败: {}", e);
        let _ = app_handle.emit_all("startup-progress", StartupEvent::error(
            "SeekDB 验证失败",
            format!("{}", e)
        ));
        return;
    }
    
    let python_path = python_env.get_python_executable();
    let python_path_str = python_path.to_str().expect("无法转换 Python 路径");
    log::info!("✅ Python 可执行文件: {}", python_path_str);
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::success(1, "Python 环境和 SeekDB 准备完成"));

    // ============================================================
    // 2. 配置文件加载
    // ============================================================
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  步骤 2/3: 加载配置文件");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::progress(2, "加载配置文件"));
    
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
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::success(2, "配置文件加载完成"));

    // ============================================================
    // 3. 初始化应用状态
    // ============================================================
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  步骤 3/3: 初始化应用状态");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let _ = app_handle.emit_all("startup-progress", StartupEvent::progress_with_details(
        3, 
        "初始化应用状态",
        "正在初始化向量数据库和AI服务..."
    ));
    
    log::info!("开始初始化应用状态...");
    
    let app_state_result = AppState::new_with_full_config(
        &db_path_str, 
        app_config, 
        model_cache_dir_str,
        Some(python_path_str)
    )
    .await;

    match app_state_result {
        Ok(app_state) => {
            // 保存到状态包装器
            let mut state_guard = state_wrapper.lock().await;
            *state_guard = Some(app_state);
            
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::info!("  ✅ 应用启动成功！");
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            let _ = app_handle.emit_all("startup-progress", StartupEvent::success(3, "应用启动成功！"));
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
            
            // 获取应用数据目录
            let app_data_dir = app
                .path_resolver()
                .app_data_dir()
                .expect("Failed to get app data directory");

            // 确保数据目录存在
            if !app_data_dir.exists() {
                fs::create_dir_all(&app_data_dir)
                    .expect("Failed to create app data directory");
            }

            // 创建数据库文件路径
            let db_path = app_data_dir.join("mine_kb.db");
            let db_path_str = db_path
                .to_str()
                .expect("Failed to convert database path to string")
                .to_string();

            log::info!("数据库文件路径: {}", db_path_str);

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

/// 加载应用配置
fn load_app_config(app_data_dir: &PathBuf) -> Option<AppConfig> {
    // 配置文件优先级：
    // 1. 应用数据目录中的 config.json
    // 2. 项目根目录的 config.json
    // 3. 环境变量

    let config_paths = vec![
        app_data_dir.join("config.json"),
        PathBuf::from("config.json"),
        PathBuf::from("../config.json"),
    ];

    for config_path in config_paths {
        if config_path.exists() {
            log::info!("尝试从配置文件读取: {:?}", config_path);
            match AppConfig::load_from_file(&config_path) {
                Ok(config) => {
                    log::info!("成功从配置文件读取配置: {:?}", config_path);
                    log::info!("  - Model: {}", config.llm.model);
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
