use crate::services::{
    project_service::ProjectService,
    document_service::DocumentService,
    conversation_service::ConversationService,
    llm_client::{LlmClient, LlmConfig as LlmClientConfig, LlmProvider},
};
use crate::config::{AppConfig, LlmConfig};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// 应用全局状态管理
pub struct AppState {
    pub project_service: Arc<Mutex<ProjectService>>,
    pub document_service: Arc<Mutex<DocumentService>>,
    pub conversation_service: Arc<Mutex<ConversationService>>,
    pub llm_client: Arc<Mutex<LlmClient>>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        // 初始化各个服务
        let document_service = Arc::new(Mutex::new(DocumentService::new().await?));

        let vector_db = {
            let doc_service = document_service.lock().await;
            doc_service.get_vector_db()
        };

        let project_service = Arc::new(Mutex::new(ProjectService::new_async(vector_db.clone()).await?));
        let conversation_service = Arc::new(Mutex::new(ConversationService::new(vector_db).await));

        // 初始化 LLM 客户端（从环境变量）
        let llm_client = Arc::new(Mutex::new(Self::create_llm_client(None)?));

        Ok(Self {
            project_service,
            document_service,
            conversation_service,
            llm_client,
        })
    }

    pub async fn new_with_db_path(db_path: &str) -> Result<Self> {
        Self::new_with_config(db_path, None, None).await
    }

    pub async fn new_with_config(db_path: &str, app_config: Option<AppConfig>, _model_cache_dir: Option<String>) -> Result<Self> {
        Self::new_with_full_config(db_path, app_config, _model_cache_dir).await
    }

    pub async fn new_with_full_config(
        db_path: &str,
        app_config: Option<AppConfig>,
        _model_cache_dir: Option<String>,
    ) -> Result<Self> {
        log::info!("📦 初始化应用状态...");
        log::info!("  - 数据库路径: {}", db_path);

        // 从配置文件或环境变量获取 API Key
        let api_key = if let Some(ref config) = app_config {
            config.llm.api_key.clone()
        } else {
            std::env::var("DASHSCOPE_API_KEY")
                .map_err(|_| anyhow!("未找到 DASHSCOPE_API_KEY，请在 config.json 配置或设置环境变量"))?
        };

        // 获取 embedding base URL（优先使用 embedding 配置，而不是 LLM 配置）
        let embedding_base_url = app_config
            .as_ref()
            .and_then(|c| c.embedding.as_ref())
            .and_then(|e| e.base_url.clone());

        // 初始化各服务（嵌入式 SeekDB）
        let t_doc = Instant::now();
        let document_service = Arc::new(Mutex::new(
            DocumentService::with_full_config(db_path, api_key, embedding_base_url).await?,
        ));
        log::info!("📦 DocumentService (含 SeekDB+Embedding) 初始化总耗时: {:?}", t_doc.elapsed());

        let vector_db = {
            let doc_service = document_service.lock().await;
            doc_service.get_vector_db()
        };

        let project_service = Arc::new(Mutex::new(ProjectService::new_async(vector_db.clone()).await?));
        let t_conv = Instant::now();
        let conversation_service = Arc::new(Mutex::new(ConversationService::new(vector_db).await));
        log::info!("📦 ConversationService 初始化总耗时: {:?}", t_conv.elapsed());

        // 初始化 LLM 客户端（使用配置文件的配置）
        let llm_config = app_config.as_ref().map(|c| c.llm.clone());
        let llm_client = Arc::new(Mutex::new(Self::create_llm_client(llm_config)?));

        log::info!("✅ 应用状态初始化完成");

        Ok(Self {
            project_service,
            document_service,
            conversation_service,
            llm_client,
        })
    }

    /// 获取项目服务的引用
    pub fn project_service(&self) -> Arc<Mutex<ProjectService>> {
        self.project_service.clone()
    }

    /// 获取文档服务的引用
    pub fn document_service(&self) -> Arc<Mutex<DocumentService>> {
        self.document_service.clone()
    }

    /// 获取对话服务的引用
    pub fn conversation_service(&self) -> Arc<Mutex<ConversationService>> {
        self.conversation_service.clone()
    }

    /// 获取 LLM 客户端的引用
    pub fn llm_client(&self) -> Arc<Mutex<LlmClient>> {
        self.llm_client.clone()
    }

    /// 创建 LLM 客户端，配置阿里百炼
    fn create_llm_client(llm_config: Option<LlmConfig>) -> Result<LlmClient> {
        let (api_key, model, base_url_opt, max_tokens, temperature, stream) = if let Some(config) = llm_config {
            // 使用配置文件
            if config.api_key.is_empty() {
                return Err(anyhow!("配置文件中的 API Key 不能为空"));
            }
            log::info!("使用配置文件中的 LLM 配置");

            let base_url = if let Some(url) = config.base_url {
                if !url.is_empty() {
                    Some(url)
                } else {
                    None
                }
            } else {
                None
            };

            (
                config.api_key,
                config.model,
                base_url,
                config.max_tokens.map(|t| t as u32),
                config.temperature.map(|t| t as f32),
                config.stream,
            )
        } else {
            // 从环境变量读取
            log::info!("尝试从环境变量读取 API Key");
            let api_key = std::env::var("DASHSCOPE_API_KEY")
                .map_err(|_| anyhow!("未找到 API Key。请在 config.json 中设置或设置环境变量 DASHSCOPE_API_KEY"))?;

            (
                api_key,
                "qwen-max".to_string(),
                None,
                Some(4000),
                Some(0.7),
                true, // 默认启用流式输出
            )
        };

        log::info!("[CHAT] LLM API Key (调试): {}", api_key);

        // 确定 Base URL
        let base_url = if let Some(url) = base_url_opt {
            log::info!("使用配置的 Base URL: {}", url);
            url
        } else {
            log::info!("Base URL 未配置，自动检测...");
            Self::get_dashscope_base_url()
        };

        log::info!("初始化 LLM 客户端:");
        log::info!("  - Provider: OpenAI Compatible (阿里百炼)");
        log::info!("  - Model: {}", model);
        log::info!("  - Base URL: {}", base_url);
        log::info!("  - Max Tokens: {:?}", max_tokens);
        log::info!("  - Temperature: {:?}", temperature);
        log::info!("  - Stream: {}", stream);

        let config = LlmClientConfig {
            provider: LlmProvider::OpenAI, // 使用 OpenAI 兼容模式
            api_key,
            model,
            base_url,
            max_tokens,
            temperature,
            stream,
        };

        LlmClient::new(config)
    }

    /// 获取阿里百炼 Base URL（根据 IP 判断国内或海外）
    fn get_dashscope_base_url() -> String {
        // 尝试检测 IP 位置，默认使用国内 endpoint
        match Self::is_china_ip() {
            Ok(true) => {
                log::info!("检测到中国 IP，使用国内 endpoint");
                "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()
            }
            Ok(false) => {
                log::info!("检测到海外 IP，使用国际 endpoint");
                "https://dashscope-intl.aliyuncs.com/compatible-mode/v1".to_string()
            }
            Err(e) => {
                log::warn!("IP 检测失败: {}，默认使用国内 endpoint", e);
                "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()
            }
        }
    }

    /// 简单的 IP 位置检测（检查是否在中国）
    fn is_china_ip() -> Result<bool> {
        // 方法1：通过访问公共 IP 检测服务
        // 这里使用一个简单的启发式方法：尝试访问中国的服务

        use std::time::Duration;
        use std::net::TcpStream;

        // 尝试连接到中国的公共 DNS 服务器（114.114.114.114）
        // 如果连接速度快（<200ms），说明可能在中国
        let start = std::time::Instant::now();
        let result = TcpStream::connect_timeout(
            &"114.114.114.114:53".parse().unwrap(),
            Duration::from_millis(200)
        );
        let china_latency = start.elapsed();

        // 尝试连接到 Google DNS（8.8.8.8）
        let start = std::time::Instant::now();
        let google_result = TcpStream::connect_timeout(
            &"8.8.8.8:53".parse().unwrap(),
            Duration::from_millis(200)
        );
        let google_latency = start.elapsed();

        // 如果能连接到 114 且速度更快，则判断为中国 IP
        if result.is_ok() && (google_result.is_err() || china_latency < google_latency) {
            log::debug!("中国DNS延迟: {:?}, Google DNS延迟: {:?}", china_latency, google_latency);
            Ok(true)
        } else {
            Ok(false)
        }
    }

}
