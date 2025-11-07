# SeekDB ORDER BY 不支持问题修复

## 问题描述

在使用 SeekDB/ObLite 作为向量数据库时，遇到以下错误：

```
[SeekDB Bridge] Query error: fetchall failed 1235 Not supported feature or function
```

错误发生在执行以下查询时：

```sql
SELECT id, project_id, document_id, chunk_index, content, embedding, metadata
FROM vector_documents
WHERE project_id = 'ac031fa7-d0e5-4585-86bd-f1cc82999924'
ORDER BY document_id, chunk_index
```

## 根本原因

SeekDB/ObLite 不支持普通的 `ORDER BY` 子句（除了向量搜索专用的 `ORDER BY l2_distance(embedding, ...) APPROXIMATE` 语法）。

当代码尝试使用 `ORDER BY` 对结果进行排序时，数据库引擎返回错误代码 1235（不支持的功能或函数）。

## 解决方案

### 修复策略

移除所有普通的 `ORDER BY` 子句，改为在 Rust 代码中进行内存排序。

### 具体修改

在 `src-tauri/src/services/seekdb_adapter.rs` 中修改了以下函数：

#### 1. `get_project_documents`

**修改前：**
```rust
let rows = subprocess.query(
    "SELECT id, project_id, document_id, chunk_index, content, embedding, metadata
     FROM vector_documents
     WHERE project_id = ?
     ORDER BY document_id, chunk_index",
    vec![Value::String(project_id.to_string())],
)?;
```

**修改后：**
```rust
// Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
let rows = subprocess.query(
    "SELECT id, project_id, document_id, chunk_index, content, embedding, metadata
     FROM vector_documents
     WHERE project_id = ?",
    vec![Value::String(project_id.to_string())],
)?;

// ... 处理行数据 ...

// Sort documents by document_id and chunk_index in memory
documents.sort_by(|a, b| {
    match a.document_id.cmp(&b.document_id) {
        std::cmp::Ordering::Equal => a.chunk_index.cmp(&b.chunk_index),
        other => other,
    }
});
```

#### 2. `load_all_projects`

**修改前：**
```sql
SELECT id, name, description, status, document_count, created_at, updated_at
FROM projects
ORDER BY updated_at DESC
```

**修改后：**
```rust
// Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
let rows = subprocess.query(
    "SELECT id, name, description, status, document_count, created_at, updated_at
     FROM projects",
    vec![],
)?;

// ... 处理行数据 ...

// Sort by updated_at DESC in memory
projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
```

#### 3. `load_conversations_by_project`

**修改前：**
```sql
SELECT id, project_id, title, created_at, updated_at, message_count
FROM conversations
WHERE project_id = ?
ORDER BY updated_at DESC
```

**修改后：**
```rust
// Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
let rows = subprocess.query(
    "SELECT id, project_id, title, created_at, updated_at, message_count
     FROM conversations
     WHERE project_id = ?",
    vec![Value::String(project_id.to_string())],
)?;

// ... 处理行数据 ...

// Sort by updated_at DESC in memory
conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
```

#### 4. `load_all_conversations`

**修改前：**
```sql
SELECT id, project_id, title, created_at, updated_at, message_count
FROM conversations
ORDER BY updated_at DESC
```

**修改后：**
```rust
// Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
let rows = subprocess.query(
    "SELECT id, project_id, title, created_at, updated_at, message_count
     FROM conversations",
    vec![],
)?;

// ... 处理行数据 ...

// Sort by updated_at DESC in memory
conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
```

#### 5. `load_messages_by_conversation`

**修改前：**
```sql
SELECT id, conversation_id, role, content, created_at, sources
FROM messages
WHERE conversation_id = ?
ORDER BY created_at ASC
```

**修改后：**
```rust
// Note: SeekDB/ObLite doesn't support ORDER BY, so we sort in memory
let rows = subprocess.query(
    "SELECT id, conversation_id, role, content, created_at, sources
     FROM messages
     WHERE conversation_id = ?",
    vec![Value::String(conversation_id.to_string())],
)?;

// ... 处理行数据 ...

// Sort by created_at ASC in memory
messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
```

## 注意事项

### 向量搜索的 ORDER BY

SeekDB/ObLite 仍然支持向量搜索专用的 `ORDER BY` 语法：

```sql
ORDER BY l2_distance(embedding, '[...]') APPROXIMATE
```

这种语法用于向量相似度搜索，是被支持的，不需要修改。

### 性能考虑

1. **内存排序的性能影响**
   - 对于小到中等规模的数据集（几千条记录），内存排序的性能影响可忽略不计
   - 如果数据量很大（数十万条以上），可能需要考虑：
     - 分页加载
     - 缓存排序结果
     - 使用其他支持 ORDER BY 的数据库

2. **排序稳定性**
   - Rust 的 `sort_by` 方法保证排序的稳定性
   - 对于相同排序键的记录，原始顺序会被保持

## 验证方法

### 1. 编译检查

```bash
cd /home/ubuntu/Desktop/mine-kb/src-tauri
cargo check
```

### 2. 运行测试

运行应用并执行以下操作：
- 创建项目并添加文档
- 在聊天界面发送消息
- 检查是否出现 `fetchall failed 1235` 错误

### 3. 检查日志

查看应用日志，确认混合检索成功：

```bash
# 启动应用并观察日志
# 应该看到类似以下输出：
# ✅ [HYBRID-SEARCH] 混合检索完成，返回 N 个相关文档块
```

## 相关文件

- `src-tauri/src/services/seekdb_adapter.rs` - 主要修改文件
- `src-tauri/python/seekdb_bridge.py` - SeekDB Python 桥接
- `src-tauri/src/commands/chat.rs` - 聊天命令（调用混合检索）

## 修复日期

2025-10-29

## 影响范围

- 项目列表查询
- 对话列表查询
- 消息历史查询
- 文档块查询
- 聊天上下文检索

所有这些功能现在都能正常工作，不再出现 `fetchall failed 1235` 错误。

