use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm: LlmConfig,
    pub embedding: Option<EmbeddingConfig>,
    pub speech: Option<SpeechConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub model: String,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    #[serde(rename = "maxTokens")]
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    #[serde(default = "default_stream")]
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechConfig {
    pub provider: String,
    pub aliyun: Option<AliyunSpeechConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliyunSpeechConfig {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub app_key: String,
}

/// 默认启用流式输出
fn default_stream() -> bool {
    true
}

impl AppConfig {
    /// 从文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow!("无法读取配置文件 {:?}: {}", path.as_ref(), e))?;

        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow!("配置文件格式错误: {}", e))?;

        config.validate()?;
        Ok(config)
    }

    /// 验证配置
    fn validate(&self) -> Result<()> {
        if self.llm.api_key.is_empty() {
            return Err(anyhow!("API Key 不能为空"));
        }
        if self.llm.model.is_empty() {
            return Err(anyhow!("模型名称不能为空"));
        }
        Ok(())
    }

    /// 创建默认配置（用于生成示例）
    pub fn default_config() -> Self {
        Self {
            llm: LlmConfig {
                api_key: "your-api-key-here".to_string(),
                model: "qwen-max".to_string(),
                base_url: None,
                max_tokens: Some(4000),
                temperature: Some(0.7),
                stream: true,
            },
            embedding: None,
            speech: None,
        }
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path.as_ref(), content)
            .map_err(|e| anyhow!("无法保存配置文件: {}", e))?;
        Ok(())
    }
}
