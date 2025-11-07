use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 阿里云百炼 Embedding 服务
/// 文档：https://help.aliyun.com/zh/dashscope/developer-reference/text-embedding-api-details
pub struct DashScopeEmbeddingService {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: EmbeddingInput,
}

#[derive(Debug, Serialize)]
struct EmbeddingInput {
    texts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    output: EmbeddingOutput,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct EmbeddingOutput {
    embeddings: Vec<EmbeddingItem>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingItem {
    text_index: usize,
    embedding: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: usize,
}

impl DashScopeEmbeddingService {
    /// 创建新的 DashScope Embedding 服务
    ///
    /// # 参数
    /// - `api_key`: 阿里云 DashScope API Key
    /// - `base_url`: 可选的 base URL，默认自动检测国内/国际
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self> {
        log::info!("🚀 初始化 DashScope Embedding 服务...");

        if api_key.is_empty() {
            return Err(anyhow!("API Key 不能为空"));
        }

        let base_url = base_url.unwrap_or_else(|| {
            // 自动检测使用国内还是国际 endpoint
            Self::get_base_url()
        });

        log::info!("  - Base URL: {}", base_url);
        log::info!("  - 模型: text-embedding-v2");

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            api_key,
            base_url,
            model: "text-embedding-v2".to_string(),
        })
    }

    /// 生成单个文本的 embedding
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f64>> {
        let embeddings = self.embed_batch(&[text.to_string()]).await?;
        embeddings.into_iter().next()
            .ok_or_else(|| anyhow!("生成 embedding 失败"))
    }

    /// 批量生成 embeddings（推荐，效率更高）
    /// 注意：DashScope API 每次最多支持 25 个文本
    /// 自动重试：遇到临时错误会自动重试最多3次，使用指数退避策略
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f64>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // 如果文本数量超过 25 个，分批处理
        if texts.len() > 25 {
            return self.embed_batch_chunked(texts, 25).await;
        }

        // 使用重试机制调用 API
        self.embed_batch_with_retry(texts, 3).await
    }

    /// 带重试机制的批量生成 embeddings
    /// 使用指数退避策略处理临时错误
    async fn embed_batch_with_retry(
        &self,
        texts: &[String],
        max_retries: u32,
    ) -> Result<Vec<Vec<f64>>> {
        let mut retries = 0;
        let mut delay = Duration::from_millis(1000); // 初始延迟 1 秒

        loop {
            log::debug!(
                "🔄 调用 DashScope API 生成 {} 个 embeddings (尝试 {}/{})",
                texts.len(),
                retries + 1,
                max_retries + 1
            );

            match self.embed_batch_internal(texts).await {
                Ok(result) => {
                    if retries > 0 {
                        log::info!("✅ 重试成功！第 {} 次尝试成功", retries + 1);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let is_retryable = Self::is_retryable_error(&e);

                    if retries < max_retries && is_retryable {
                        log::warn!(
                            "⚠️  Embedding API 调用失败 (第 {}/{} 次)，{}ms 后重试: {}",
                            retries + 1,
                            max_retries,
                            delay.as_millis(),
                            e
                        );

                        tokio::time::sleep(delay).await;

                        // 指数退避：每次延迟翻倍，最大 30 秒
                        delay = std::cmp::min(delay * 2, Duration::from_secs(30));
                        retries += 1;
                    } else {
                        if !is_retryable {
                            log::error!("❌ 不可重试的错误: {}", e);
                        } else {
                            log::error!("❌ 达到最大重试次数 ({}次)，放弃重试", max_retries);
                        }
                        return Err(e);
                    }
                }
            }
        }
    }

    /// 内部方法：实际调用 API（不包含重试逻辑）
    async fn embed_batch_internal(&self, texts: &[String]) -> Result<Vec<Vec<f64>>> {
        let request_body = EmbeddingRequest {
            model: self.model.clone(),
            input: EmbeddingInput {
                texts: texts.to_vec(),
            },
        };

        let url = format!("{}/services/embeddings/text-embedding/text-embedding", self.base_url);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("DashScope API 调用失败 [{}]: {}", status, error_text));
        }

        let result: EmbeddingResponse = response.json().await?;

        log::debug!("✅ 成功生成 {} 个 embeddings，消耗 tokens: {}",
            result.output.embeddings.len(),
            result.usage.total_tokens
        );

        // 按 text_index 排序并提取 embedding
        let mut embeddings: Vec<_> = result.output.embeddings;
        embeddings.sort_by_key(|e| e.text_index);

        Ok(embeddings.into_iter().map(|e| e.embedding).collect())
    }

    /// 判断错误是否可重试
    /// 可重试的错误包括：网络超时、429限流、5xx服务器错误
    fn is_retryable_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();

        // 网络相关错误
        if error_str.contains("timeout")
            || error_str.contains("connection")
            || error_str.contains("network") {
            return true;
        }

        // HTTP 状态码相关
        if error_str.contains("[429]")  // 限流
            || error_str.contains("[500]")  // 服务器内部错误
            || error_str.contains("[502]")  // 网关错误
            || error_str.contains("[503]")  // 服务不可用
            || error_str.contains("[504]") {  // 网关超时
            return true;
        }

        false
    }

    /// 分块批量处理（当文本数量超过 API 限制时）
    /// 每个分块都会使用重试机制
    async fn embed_batch_chunked(&self, texts: &[String], chunk_size: usize) -> Result<Vec<Vec<f64>>> {
        log::debug!("📦 分 {} 批处理 {} 个文本",
            (texts.len() + chunk_size - 1) / chunk_size,
            texts.len()
        );

        let mut all_embeddings = Vec::new();

        for (i, chunk) in texts.chunks(chunk_size).enumerate() {
            log::debug!("处理第 {}/{} 批 ({} 个文本)",
                i + 1,
                (texts.len() + chunk_size - 1) / chunk_size,
                chunk.len()
            );

            // 每个分块都使用重试机制
            let chunk_embeddings = self.embed_batch_with_retry(chunk, 3).await?;
            all_embeddings.extend(chunk_embeddings);
        }

        Ok(all_embeddings)
    }

    /// 获取 embedding 维度
    /// text-embedding-v2: 1536 维
    /// text-embedding-v1: 1536 维
    pub fn embedding_dim(&self) -> usize {
        1536
    }

    /// 获取 base URL（自动检测国内/国际）
    fn get_base_url() -> String {
        // 默认使用国内 endpoint
        // 如果用户在海外，可以通过配置文件指定国际 endpoint
        "https://dashscope.aliyuncs.com/api/v1".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要 API Key
    async fn test_dashscope_embedding() {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .expect("需要设置 DASHSCOPE_API_KEY 环境变量");

        let service = DashScopeEmbeddingService::new(api_key, None).unwrap();

        let text = "这是一个测试文本";
        let embedding = service.embed_text(text).await.unwrap();

        assert_eq!(embedding.len(), 1536);

        // 验证向量不全为零
        let sum: f64 = embedding.iter().sum();
        assert!(sum.abs() > 0.0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_batch_embedding() {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .expect("需要设置 DASHSCOPE_API_KEY 环境变量");

        let service = DashScopeEmbeddingService::new(api_key, None).unwrap();

        let texts = vec![
            "第一个文本".to_string(),
            "第二个文本".to_string(),
            "第三个文本".to_string(),
        ];

        let embeddings = service.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), texts.len());

        for embedding in embeddings {
            assert_eq!(embedding.len(), 1536);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_semantic_similarity() {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .expect("需要设置 DASHSCOPE_API_KEY 环境变量");

        let service = DashScopeEmbeddingService::new(api_key, None).unwrap();

        let text1 = "我喜欢吃苹果";
        let text2 = "我喜欢吃水果";
        let text3 = "今天天气很好";

        let emb1 = service.embed_text(text1).await.unwrap();
        let emb2 = service.embed_text(text2).await.unwrap();
        let emb3 = service.embed_text(text3).await.unwrap();

        fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
            let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
            let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
            dot / (norm_a * norm_b)
        }

        let sim_12 = cosine_similarity(&emb1, &emb2);
        let sim_13 = cosine_similarity(&emb1, &emb3);

        println!("相似文本相似度: {:.4}", sim_12);
        println!("不相似文本相似度: {:.4}", sim_13);

        assert!(sim_12 > sim_13, "相似文本应该有更高的相似度");
        assert!(sim_12 > 0.5, "相似文本相似度应该 > 0.5");
    }
}
