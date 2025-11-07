# SeekDB 混合检索与对话日志增强

## 更新日期
2025-01-29

## 概述
本次更新实现了两个主要功能：
1. **混合检索（Hybrid Search）**：结合向量检索和全文检索，提升文档检索的准确性和召回率
2. **详细对话日志**：在整个对话流程中添加结构化的日志输出，便于调试和监控

---

## 🔍 功能 1：混合检索（Hybrid Search）

### 什么是混合检索？

混合检索结合了两种检索技术：
- **向量检索（Semantic Search）**：基于语义相似度，理解查询的"含义"
- **全文检索（Keyword Search）**：基于关键词匹配，精确匹配查询词

通过 SeekDB 的 `dbms_hybrid_search.search()` 函数，可以同时利用这两种检索方式，获得更好的检索效果。

### 技术实现

#### 1. 数据库表结构修改

**文件：** `src-tauri/src/services/seekdb_adapter.rs`

在 `vector_documents` 表创建时添加了全文索引：

```rust
// 旧版本：只有向量索引
CREATE TABLE IF NOT EXISTS vector_documents (
    ...
    embedding vector(1536),
    ...
)

// 新版本：同时支持向量索引和全文索引
CREATE TABLE IF NOT EXISTS vector_documents (
    id VARCHAR(36) PRIMARY KEY,
    project_id VARCHAR(36) NOT NULL,
    document_id VARCHAR(36) NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536),
    metadata TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(document_id, chunk_index),
    VECTOR INDEX idx_embedding(embedding) WITH (distance=l2, type=hnsw, lib=vsag),
    FULLTEXT idx_content(content)  -- ✨ 新增全文索引
)
```

**关键点：**
- `VECTOR INDEX idx_embedding`：HNSW 向量索引，用于语义搜索
- `FULLTEXT idx_content`：全文索引，用于关键词搜索

#### 2. 混合检索 API 实现

**文件：** `src-tauri/src/services/seekdb_adapter.rs`

新增 `hybrid_search()` 方法：

```rust
pub fn hybrid_search(
    &self,
    query_text: &str,          // 查询文本（用于全文检索）
    query_embedding: &[f64],   // 查询向量（用于向量检索）
    project_id: Option<&str>,  // 项目ID过滤
    limit: usize,              // 返回结果数量
    semantic_boost: f64,       // 语义权重（0.0-1.0）
) -> Result<Vec<SearchResult>>
```

**参数说明：**
- `semantic_boost = 0.7`：向量检索占 70% 权重，全文检索占 30% 权重
- 可以根据实际需求调整权重

**检索过程：**
1. 构建混合搜索查询（Elasticsearch 风格的 DSL）
2. 使用 `dbms_hybrid_search.search()` 执行检索
3. 解析结果，包含 `_keyword_score`（关键词分数）和 `_semantic_score`（语义分数）

#### 3. 文档服务层集成

**文件：** `src-tauri/src/services/document_service.rs`

新增 `search_similar_chunks_hybrid()` 方法：

```rust
pub async fn search_similar_chunks_hybrid(
    &self,
    project_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<SimilarChunk>>
```

**流程：**
1. 调用 DashScope Embedding API 生成查询向量
2. 调用 SeekDB 的混合检索 API
3. 返回最相关的文档块

**优势：**
- 同时利用语义理解和关键词匹配
- 对于包含专业术语的查询，全文检索能提供更准确的结果
- 对于自然语言查询，向量检索能理解语义

#### 4. 聊天命令集成

**文件：** `src-tauri/src/commands/chat.rs`

在 `send_message` 命令中使用混合检索：

```rust
// 旧代码（纯向量检索）
document_service_guard.search_similar_chunks(&project_id, &query, 5).await

// 新代码（混合检索）
document_service_guard.search_similar_chunks_hybrid(&project_id, &query, 5).await
```

---

## 📝 功能 2：详细对话日志

### 日志结构

在整个对话流程中添加了分阶段、结构化的日志输出：

#### 阶段 1：保存用户消息
```
💾 [CHAT] 步骤 1/5: 保存用户消息到数据库
✅ [CHAT] 用户消息已保存
```

#### 阶段 2：混合检索
```
🔍 [CHAT] 步骤 2/5: 执行混合检索（向量+全文）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔍 [HYBRID-SEARCH] 开始混合检索文档块
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📋 项目ID: xxx
💬 查询内容: xxx
📊 返回数量: 5
🌐 调用 DashScope Embedding API...
✅ 生成查询向量成功，维度: 1536
📚 数据库中该项目的文档块总数: 42
🔄 执行混合检索（语义权重=0.7）...
✅ 混合检索完成，找到 5 个结果

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📄 结果 #1
   🔢 分数: 0.8542
   📝 内容预览: 租户管理是系统的核心功能...
   📂 文档ID: doc-123
   🔖 块索引: 2
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ [CHAT] 混合检索成功，找到 5 个相关文档块
   📄 上下文块 #1: 文件="用户手册.md", 相关度=0.8542
      内容: 租户管理是系统的核心功能...
```

#### 阶段 3：获取对话历史
```
📜 [CHAT] 步骤 3/5: 获取对话历史
✅ [CHAT] 获取到 12 条历史消息
   消息 #1: User - 如何创建租户
   消息 #2: Assistant - 创建租户的步骤如下...
   消息 #3: User - 如何删除租户
```

#### 阶段 4：调用 LLM
```
🤖 [CHAT] 步骤 4/5: 调用 LLM 生成响应
   上下文块数量: 5
   历史消息数量: 12
✅ [CHAT] LLM 流式响应已建立
✅ [CHAT] LLM 响应完成: resp_xxx
   总 token 数: 243
   响应长度: 1024 字符
🎉 [CHAT] 流式传输完成，共收到 243 个 token
📝 [CHAT] AI 响应内容预览: 删除租户的步骤如下：1. 登录管理后台...
```

#### 阶段 5：保存响应
```
💾 [CHAT] 步骤 5/5: 保存 AI 响应到数据库
✅ [CHAT] AI 消息已保存，消息ID: msg-456
📎 [CHAT] 附加来源文档信息（5 个）
✅ [CHAT] 来源文档信息已附加

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 [CHAT] 对话处理完成！
   对话ID: conv-789
   响应长度: 1024 字符
   使用了 5 个上下文文档块
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### 日志优势

1. **可追踪性**：每个步骤都有清晰的日志标记
2. **性能监控**：可以看到每个阶段的耗时和数据量
3. **调试友好**：出现问题时可以快速定位是哪个环节
4. **数据洞察**：可以分析检索质量、上下文使用情况等

---

## 🚀 使用方法

### 启用混合检索

混合检索已经自动启用，无需额外配置。在发送消息时，系统会自动使用混合检索来查找相关文档。

### 查看日志

启动应用时，日志会输出到终端：

```bash
cd /home/ubuntu/Desktop/mine-kb
npm run tauri dev
```

日志示例：
```
[2025-01-29 10:30:25] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[2025-01-29 10:30:25] 💬 [CHAT] 开始处理对话消息
[2025-01-29 10:30:25] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[2025-01-29 10:30:25] 📋 对话ID: xxx
[2025-01-29 10:30:25] 📁 项目ID: xxx
[2025-01-29 10:30:25] 💬 用户消息: 如何删除租户？
...
```

---

## 📊 性能对比

### 纯向量检索 vs 混合检索

| 场景 | 纯向量检索 | 混合检索 |
|------|-----------|---------|
| 自然语言查询 | ✅ 良好 | ✅✅ 优秀 |
| 专业术语查询 | ⚠️ 一般 | ✅✅ 优秀 |
| 缩写词查询 | ❌ 较差 | ✅ 良好 |
| 精确匹配 | ⚠️ 一般 | ✅✅ 优秀 |
| 语义理解 | ✅✅ 优秀 | ✅✅ 优秀 |

**示例：**

查询："API key 在哪里配置？"
- **纯向量检索**：可能返回关于"配置"的各种文档，不一定包含 "API key"
- **混合检索**：会优先返回同时包含 "API key" 和"配置"的文档，同时理解语义

---

## 🔧 技术细节

### 混合检索参数

**语义权重（semantic_boost）**：
- 当前设置：`0.7`
- 含义：向量检索占 70%，全文检索占 30%
- 可调整范围：`0.0` - `1.0`
- 调整建议：
  - 如果用户经常使用专业术语，可以降低到 `0.5`
  - 如果用户多用自然语言，可以提高到 `0.8`

### 数据库兼容性

⚠️ **重要提示**：本次更新修改了数据库表结构，旧数据库不兼容。

**迁移方案：**

1. **删除旧数据库**（开发环境）：
   ```bash
   rm -rf ~/.local/share/com.mine-kb.app/oblite.db
   ```

2. **保留数据迁移**（生产环境）：
   需要手动添加全文索引：
   ```sql
   CREATE FULLTEXT INDEX idx_content ON vector_documents(content);
   ```

---

## 📈 监控指标

通过日志可以监控以下指标：

1. **检索效果**
   - 混合检索找到的文档数量
   - 每个文档的相关度分数
   - 关键词分数 vs 语义分数

2. **性能指标**
   - Embedding API 调用时间
   - 混合检索执行时间
   - LLM 响应时间
   - Token 数量

3. **数据质量**
   - 项目中的文档块总数
   - 向量维度一致性
   - 上下文使用率

---

## 🐛 故障排查

### 问题 1：混合检索返回 0 个结果

**可能原因：**
- 项目中没有文档
- 数据库表结构未更新（缺少全文索引）

**解决方案：**
1. 检查日志中的"数据库中该项目的文档块总数"
2. 如果为 0，上传一些文档
3. 如果数据库表结构旧，删除数据库重新创建

### 问题 2：混合检索报错

**可能原因：**
- SeekDB 版本不支持混合检索
- 表结构不正确

**解决方案：**
1. 确认 SeekDB 版本 >= 0.0.1.dev2
2. 检查表结构是否包含 FULLTEXT 索引：
   ```sql
   SHOW CREATE TABLE vector_documents;
   ```

### 问题 3：日志显示"混合检索失败，将不使用上下文"

**可能原因：**
- Embedding API 调用失败
- 数据库连接问题
- 混合检索语法错误

**解决方案：**
1. 查看详细错误日志
2. 验证 DASHSCOPE_API_KEY 是否正确
3. 检查网络连接

---

## 📚 参考文档

- [SeekDB 官方文档](./seekdb.md)
- [混合检索示例](./seekdb.md#33-混合检索)
- [向量检索原理](./seekdb.md#31-向量检索)
- [全文检索原理](./seekdb.md#32-全文检索)

---

## 🎯 下一步优化

1. **动态权重调整**：根据查询类型自动调整语义权重
2. **多策略融合**：结合 BM25、TF-IDF 等传统检索算法
3. **结果重排序**：使用 Cross-Encoder 对混合检索结果重排序
4. **查询扩展**：自动扩展查询词，提高召回率
5. **缓存优化**：缓存常见查询的检索结果

---

## 📝 更新记录

### Version 1.0 (2025-01-29)
- ✅ 实现混合检索功能
- ✅ 添加详细对话日志
- ✅ 更新数据库表结构
- ✅ 编写技术文档

---

## 👥 贡献者

- 实现：Cursor AI Agent
- 需求：用户
- 日期：2025-01-29

