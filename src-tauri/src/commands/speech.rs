use tauri::command;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};
use crate::config::AppConfig;
use crate::services::speech_service::AliyunAsrService;

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechConfig {
    pub configured: bool,
    pub provider: Option<String>,
    pub message: Option<String>,
}

/// 检查语音识别配置
#[command]
pub async fn check_speech_config() -> Result<SpeechConfig, String> {
    match load_speech_config().await {
        Ok((provider, _)) => Ok(SpeechConfig {
            configured: true,
            provider: Some(provider),
            message: Some("语音识别服务已配置".to_string()),
        }),
        Err(e) => Ok(SpeechConfig {
            configured: false,
            provider: None,
            message: Some(e),
        }),
    }
}

/// 语音识别（使用云服务）
#[command]
pub async fn recognize_speech(
    audio_data: String,
    audio_format: String,
) -> Result<String, String> {
    println!("收到语音识别请求");
    println!("音频格式: {}", audio_format);
    println!("Base64数据长度: {}", audio_data.len());

    // 解码 Base64
    let audio_bytes = general_purpose::STANDARD
        .decode(&audio_data)
        .map_err(|e| format!("Base64解码失败: {}", e))?;

    println!("解码后音频大小: {} bytes", audio_bytes.len());

    // 加载配置
    let (provider, config) = load_speech_config().await
        .map_err(|e| format!("配置错误: {}", e))?;

    match provider.as_str() {
        "aliyun" => {
            let speech_config = config.speech.ok_or("语音配置不存在")?;
            let aliyun_config = speech_config.aliyun.ok_or("阿里云配置不存在")?;

            let mut service = AliyunAsrService::new(
                aliyun_config.access_key_id,
                aliyun_config.access_key_secret,
                aliyun_config.app_key,
            );

            service.recognize_speech(&audio_bytes).await
                .map_err(|e| format!("语音识别失败: {}", e))
        }
        _ => Err(format!("不支持的语音服务提供商: {}", provider)),
    }
}

async fn load_speech_config() -> Result<(String, AppConfig), String> {
    let config_path = std::env::current_dir()
        .map_err(|e| format!("获取当前目录失败: {}", e))?
        .join("config.json");

    let config = AppConfig::load_from_file(&config_path)
        .map_err(|e| format!("加载配置文件失败: {}", e))?;

    let speech_config = config.speech.as_ref()
        .ok_or("配置文件中未找到语音配置")?;

    let provider = speech_config.provider.clone();

    match provider.as_str() {
        "aliyun" => {
            if speech_config.aliyun.is_none() {
                return Err("阿里云配置不存在".to_string());
            }
        }
        _ => return Err(format!("不支持的语音服务提供商: {}", provider)),
    }

    Ok((provider, config))
}
