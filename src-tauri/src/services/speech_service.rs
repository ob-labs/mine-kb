use anyhow::{Result, anyhow};
use serde_json::Value;
use std::time::Duration;
use std::collections::BTreeMap;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use base64::{Engine as _, engine::general_purpose};

type HmacSha1 = Hmac<Sha1>;

/// 阿里云智能语音服务 - 一句话识别
pub struct AliyunAsrService {
    access_key_id: String,
    access_key_secret: String,
    app_key: String,
    token_cache: Option<(String, std::time::Instant)>,
}

impl AliyunAsrService {
    pub fn new(access_key_id: String, access_key_secret: String, app_key: String) -> Self {
        Self {
            access_key_id,
            access_key_secret,
            app_key,
            token_cache: None,
        }
    }

    pub async fn recognize_speech(&mut self, audio_data: &[u8]) -> Result<String> {
        println!("阿里云智能语音服务开始识别，音频大小: {} bytes", audio_data.len());

        // 获取Token（使用正确的RPC签名方式）
        let token = self.get_token().await?;

        // 使用Token调用识别API
        self.call_recognition_api(&token, audio_data).await
    }

    /// 获取Token（使用标准RPC签名 - CreateToken）
    async fn get_token(&mut self) -> Result<String> {
        // 检查缓存是否有效（Token有效期24小时，提前1小时刷新）
        if let Some((cached_token, cached_time)) = &self.token_cache {
            if cached_time.elapsed() < Duration::from_secs(23 * 3600) {
                println!("使用缓存的Token");
                return Ok(cached_token.clone());
            }
        }

        println!("获取新Token（使用RPC签名 - CreateToken）");

        // 构造参数（使用BTreeMap自动排序）
        let mut params = BTreeMap::new();
        params.insert("Action".to_string(), "CreateToken".to_string());  // 改为CreateToken
        params.insert("Version".to_string(), "2019-02-28".to_string());
        params.insert("Format".to_string(), "JSON".to_string());
        params.insert("RegionId".to_string(), "cn-shanghai".to_string());
        params.insert("AccessKeyId".to_string(), self.access_key_id.clone());
        params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
        params.insert("SignatureVersion".to_string(), "1.0".to_string());
        params.insert("SignatureNonce".to_string(), uuid::Uuid::new_v4().to_string());

        // 时间戳（ISO 8601格式）
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        params.insert("Timestamp".to_string(), timestamp);

        // 计算签名
        let signature = self.compute_rpc_signature("POST", &params)?;  // 改为POST
        params.insert("Signature".to_string(), signature);

        // 构建请求URL
        let query_string = self.build_canonical_query_string(&params);
        let url = format!("https://nls-meta.cn-shanghai.aliyuncs.com/?{}", query_string);

        println!("Token请求URL长度: {}", url.len());

        let client = reqwest::Client::new();
        let response = client
            .post(&url)  // 改为POST
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| anyhow!("获取Token请求失败: {}", e))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| anyhow!("读取Token响应失败: {}", e))?;

        println!("Token响应状态: {}", status);
        println!("Token响应内容: {}", response_text);

        if !status.is_success() {
            return Err(anyhow!("获取Token失败 ({}): {}", status, response_text));
        }

        // 解析响应
        let json: Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("解析Token响应失败: {}", e))?;

        // 检查错误
        if let Some(code) = json.get("Code") {
            let message = json.get("Message")
                .and_then(|m| m.as_str())
                .unwrap_or("未知错误");
            return Err(anyhow!("获取Token失败 [{}]: {}", code, message));
        }

        // 提取Token（尝试多种可能的路径）
        let token = if let Some(id) = json.get("Token").and_then(|t| t.get("Id")).and_then(|id| id.as_str()) {
            id.to_string()
        } else if let Some(token_str) = json.get("Token").and_then(|t| t.get("UserId")).and_then(|id| id.as_str()) {
            token_str.to_string()
        } else if let Some(token_str) = json.get("Token").and_then(|t| t.as_str()) {
            token_str.to_string()
        } else {
            return Err(anyhow!("Token响应中未找到Token字段。完整响应: {}", response_text));
        };

        println!("Token获取成功: {}...", &token[..std::cmp::min(20, token.len())]);

        // 缓存Token
        self.token_cache = Some((token.clone(), std::time::Instant::now()));

        Ok(token)
    }

    /// 计算阿里云RPC风格签名
    fn compute_rpc_signature(&self, method: &str, params: &BTreeMap<String, String>) -> Result<String> {
        // 1. 构建规范化查询字符串
        let canonical_query = self.build_canonical_query_string(params);

        // 2. 构建待签名字符串：METHOD&编码后的"/"&编码后的查询字符串
        let string_to_sign = format!(
            "{}&{}&{}",
            method,
            Self::percent_encode("/"),
            Self::percent_encode(&canonical_query)
        );

        println!("待签名字符串: {}", string_to_sign);

        // 3. 使用AccessKeySecret+"&"作为密钥计算HMAC-SHA1
        let key = format!("{}&", self.access_key_secret);
        let mut mac = HmacSha1::new_from_slice(key.as_bytes())
            .map_err(|e| anyhow!("创建HMAC失败: {}", e))?;
        mac.update(string_to_sign.as_bytes());
        let signature_bytes = mac.finalize().into_bytes();

        // 4. Base64编码
        let signature = general_purpose::STANDARD.encode(signature_bytes);

        println!("签名结果: {}", signature);

        Ok(signature)
    }

    /// 构建规范化查询字符串
    fn build_canonical_query_string(&self, params: &BTreeMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", Self::percent_encode(k), Self::percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// URL编码（符合阿里云规范）
    fn percent_encode(input: &str) -> String {
        urlencoding::encode(input)
            .replace("+", "%20")
            .replace("*", "%2A")
            .replace("%7E", "~")
    }

    /// 调用一句话识别API
    async fn call_recognition_api(&self, token: &str, audio_data: &[u8]) -> Result<String> {
        let url = "https://nls-gateway.cn-shanghai.aliyuncs.com/stream/v1/asr";

        println!("调用一句话识别API");
        println!("使用Token: {}...", &token[..std::cmp::min(20, token.len())]);
        println!("音频大小: {} bytes", audio_data.len());

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .header("Content-Type", "application/octet-stream")
            .header("X-NLS-Token", token)
            .query(&[
                ("appkey", self.app_key.as_str()),
                ("format", "pcm"),
                ("sample_rate", "16000"),
                ("enable_intermediate_result", "false"),
                ("enable_punctuation_prediction", "true"),
                ("enable_inverse_text_normalization", "true"),
            ])
            .body(audio_data.to_vec())
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| anyhow!("发送识别请求失败: {}", e))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| anyhow!("读取识别响应失败: {}", e))?;

        println!("识别响应状态: {}", status);
        println!("识别响应内容: {}", response_text);

        if !status.is_success() {
            return Err(anyhow!("识别请求失败 ({}): {}", status, response_text));
        }

        // 解析识别结果
        let json: Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("解析识别响应失败: {}", e))?;

        // 检查错误
        if let Some(error_code) = json.get("status") {
            if error_code.as_i64() != Some(20000000) && error_code.as_i64() != Some(0) {
                let error_msg = json.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("未知错误");
                return Err(anyhow!("识别失败 [{}]: {}", error_code, error_msg));
            }
        }

        // 提取识别结果
        let result = json
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or_else(|| anyhow!("响应中未找到识别结果"))?
            .to_string();

        println!("识别成功: {}", result);
        Ok(result)
    }
}
