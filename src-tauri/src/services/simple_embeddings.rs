use anyhow::Result;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// 简单的文本嵌入服务，基于TF-IDF实现
#[derive(Debug, Clone)]
pub struct SimpleEmbeddingService {
    vocabulary: HashMap<String, usize>,
    idf_scores: HashMap<String, f64>,
    embedding_dim: usize,
}

impl SimpleEmbeddingService {
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            vocabulary: HashMap::new(),
            idf_scores: HashMap::new(),
            embedding_dim,
        }
    }

    /// 训练词汇表和IDF分数
    pub fn train(&mut self, documents: &[String]) -> Result<()> {
        // 构建词汇表
        let mut word_doc_count: HashMap<String, usize> = HashMap::new();
        let total_docs = documents.len() as f64;

        for doc in documents {
            let words = self.tokenize(doc);
            let unique_words: std::collections::HashSet<String> = words.into_iter().collect();

            for word in unique_words {
                *word_doc_count.entry(word.clone()).or_insert(0) += 1;
                if !self.vocabulary.contains_key(&word) {
                    let idx = self.vocabulary.len();
                    self.vocabulary.insert(word.clone(), idx);
                }
            }
        }

        // 计算IDF分数
        for (word, doc_count) in word_doc_count {
            let idf = (total_docs / doc_count as f64).ln();
            self.idf_scores.insert(word, idf);
        }

        Ok(())
    }

    /// 生成文本的嵌入向量
    pub fn embed_text(&self, text: &str) -> Result<Vec<f64>> {
        let words = self.tokenize(text);
        let word_counts = self.count_words(&words);
        let total_words = words.len() as f64;

        // 计算TF-IDF向量
        let mut tfidf_vector = vec![0.0; self.embedding_dim];

        for (word, count) in word_counts {
            if let (Some(&_vocab_idx), Some(&idf)) = (self.vocabulary.get(&word), self.idf_scores.get(&word)) {
                let tf = count as f64 / total_words;
                let tfidf = tf * idf;

                // 将TF-IDF值映射到固定维度的向量
                let hash_idx = self.hash_word_to_index(&word);
                tfidf_vector[hash_idx] += tfidf;
            }
        }

        // 归一化向量
        self.normalize_vector(&mut tfidf_vector);

        Ok(tfidf_vector)
    }

    /// 生成简单的随机嵌入（用于测试）
    pub fn embed_text_simple(&self, text: &str) -> Result<Vec<f64>> {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        let mut embedding = Vec::with_capacity(self.embedding_dim);
        let mut rng_state = seed;

        for _ in 0..self.embedding_dim {
            // 简单的线性同余生成器
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let normalized = (rng_state as f64) / (u64::MAX as f64);
            embedding.push(normalized * 2.0 - 1.0); // 范围 [-1, 1]
        }

        // 归一化
        self.normalize_vector(&mut embedding);

        Ok(embedding)
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|word| {
                // 移除标点符号
                word.chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                    .collect::<String>()
                    .trim()
                    .to_string()
            })
            .filter(|word| !word.is_empty() && word.len() > 2) // 过滤短词
            .collect()
    }

    fn count_words(&self, words: &[String]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for word in words {
            *counts.entry(word.clone()).or_insert(0) += 1;
        }
        counts
    }

    fn hash_word_to_index(&self, word: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        (hasher.finish() as usize) % self.embedding_dim
    }

    fn normalize_vector(&self, vector: &mut [f64]) {
        let magnitude: f64 = vector.iter().map(|x| x * x).sum::<f64>().sqrt();
        if magnitude > 0.0 {
            for x in vector.iter_mut() {
                *x /= magnitude;
            }
        }
    }

    pub fn get_embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    pub fn get_vocabulary_size(&self) -> usize {
        self.vocabulary.len()
    }
}

impl Default for SimpleEmbeddingService {
    fn default() -> Self {
        Self::new(384) // 默认384维，类似于sentence-transformers的小模型
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_embedding_service() {
        let mut service = SimpleEmbeddingService::new(100);

        let documents = vec![
            "这是第一个测试文档".to_string(),
            "这是第二个测试文档".to_string(),
            "完全不同的内容".to_string(),
        ];

        service.train(&documents).unwrap();

        let embedding1 = service.embed_text("这是测试文档").unwrap();
        let embedding2 = service.embed_text("完全不同的内容").unwrap();

        assert_eq!(embedding1.len(), 100);
        assert_eq!(embedding2.len(), 100);

        // 验证向量已归一化
        let magnitude1: f64 = embedding1.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((magnitude1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_simple_embedding_deterministic() {
        let service = SimpleEmbeddingService::new(50);

        let text = "测试文本";
        let embedding1 = service.embed_text_simple(text).unwrap();
        let embedding2 = service.embed_text_simple(text).unwrap();

        // 相同输入应该产生相同输出
        assert_eq!(embedding1, embedding2);
        assert_eq!(embedding1.len(), 50);
    }

    #[test]
    fn test_tokenization() {
        let service = SimpleEmbeddingService::new(10);
        let tokens = service.tokenize("Hello, world! This is a test.");

        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // 短词应该被过滤掉
        assert!(!tokens.contains(&"a".to_string()));
    }
}
