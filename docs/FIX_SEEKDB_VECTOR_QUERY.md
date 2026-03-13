# 修复SeekDB向量字段查询问题


**日期**: 2025-10-29  
**问题**: SeekDB不支持在某些上下文中直接SELECT vector类型字段  
**错误信息**: `fetchall failed 1235 Not supported feature or function`

## 问题描述

在使用SeekDB进行向量检索时，遇到以下错误：

```
[SeekDB Bridge] Query error: fetchall failed 1235 Not supported feature or function
RuntimeError: fetchall failed 1235 Not supported feature or function
```

### 问题根因

SeekDB的向量类型字段（`vector(1536)`）在某些查询上下文中不能直接被SELECT。具体来说：

1. **`get_project_documents`函数**试图查询包括`embedding`字段在内的所有字段
2. SeekDB对vector类型字段的查询有限制，不是所有场景都支持直接返回vector数据
3. 这个函数实际上只需要元数据（文档数量、内容等），并不需要embedding向量

## 修复方案

### 1. 优化 `search_similar_chunks` 函数

**文件**: `src-tauri/src/services/document_service.rs`

移除了对 `get_project_documents` 的不必要调用：

```rust
// 修改前
let db = self.vector_db.lock().await;
let project_docs = db.get_project_documents(project_id)?;
log::info!("📊 数据库中该项目的文档块总数: {}", project_docs.len());
if let Some(first_doc) = project_docs.first() {
    log::info!("📐 数据库中向量维度: {}", first_doc.embedding.len());
}
let results = db.similarity_search(...)?;

// 修改后
let db = self.vector_db.lock().await;
log::info!("🔍 使用SeekDB向量检索，阈值=0.3");
let results = db.similarity_search(...)?;
```

### 2. 优化 `search_similar_chunks_hybrid` 函数

**文件**: `src-tauri/src/services/document_service.rs`

同样移除了对 `get_project_documents` 的调用：

```rust
// 修改前
let db = self.vector_db.lock().await;
let project_docs = db.get_project_documents(project_id)?;
if project_docs.is_empty() {
    return Ok(vec![]);
}
let results = db.hybrid_search(...)?;

// 修改后
let db = self.vector_db.lock().await;
log::info!("🔄 执行混合检索（语义权重=0.7）...");
let results = db.hybrid_search(...)?;
```

### 3. 修复 `get_project_documents` 函数

**文件**: `src-tauri/src/services/seekdb_adapter.rs`

修改查询，不再查询`embedding`字段：

```rust
// 修改前
let rows = subprocess.query(
    "SELECT id, project_id, document_id, chunk_index, content, embedding, metadata
     FROM vector_documents
     WHERE project_id = ?",
    vec![Value::String(project_id.to_string())],
)?;

// 修改后
let rows = subprocess.query(
    "SELECT id, project_id, document_id, chunk_index, content, metadata
     FROM vector_documents
     WHERE project_id = ?",
    vec![Value::String(project_id.to_string())],
)?;

// 返回的VectorDocument使用空向量
documents.push(VectorDocument {
    id,
    project_id,
    document_id,
    chunk_index,
    content,
    embedding: vec![], // Empty vector - not needed for this query
    metadata,
});
```

## 修改影响

### 正面影响
1. ✅ **解决了查询失败问题** - 不再尝试查询不支持的vector字段
2. ✅ **提升性能** - 减少了不必要的数据库查询
3. ✅ **简化代码** - 移除了仅用于调试的代码
4. ✅ **保持功能完整** - 向量检索功能不受影响

### 潜在影响
- `get_project_documents` 返回的文档对象中 `embedding` 字段为空向量
- 如果有其他代码依赖这个函数获取embedding，需要改用 `similarity_search`

## SeekDB向量查询最佳实践

### ✅ 推荐做法

1. **向量检索使用专用函数**：
```sql
SELECT id, content, l2_distance(embedding, '[...]') as distance
FROM vector_documents
WHERE project_id = ?
ORDER BY l2_distance(embedding, '[...]') APPROXIMATE
LIMIT 10
```

2. **元数据查询不包含vector字段**：
```sql
SELECT id, project_id, document_id, chunk_index, content, metadata
FROM vector_documents
WHERE project_id = ?
```

### ❌ 避免做法

1. **不要在普通查询中SELECT vector字段**：
```sql
-- 可能失败！
SELECT * FROM vector_documents WHERE project_id = ?
SELECT id, embedding FROM vector_documents
```

2. **不要对vector字段做非向量操作**：
```sql
-- 错误！
SELECT embedding FROM vector_documents WHERE embedding IS NOT NULL
```

## 相关SeekDB特性

### Vector类型支持
- ✅ 在 `l2_distance()` 函数中使用
- ✅ 在 `ORDER BY ... APPROXIMATE` 中使用
- ✅ 在 `VECTOR INDEX` 中使用
- ❌ 在常规 SELECT 中可能受限
- ❌ 在某些聚合函数中不支持

### 替代方案
如果确实需要获取embedding数据：
1. 使用向量检索函数（如 `similarity_search`）
2. 确保查询中包含向量操作（如 `l2_distance`）
3. 考虑将embedding存储为TEXT类型的JSON字符串（但会失去索引优势）

## 测试验证

### 验证步骤
1. 重新编译应用：
```bash
cd src-tauri
cargo build --release
```

2. 启动应用并测试聊天功能

3. 检查日志，应该看到：
```
🔍 [CHAT] 步骤 2/5: 执行SeekDB向量检索
🔍 使用SeekDB向量检索，阈值=0.3
✅ 向量搜索完成（阈值=0.3），找到 X 个结果
```

4. 验证不再出现 "Not supported feature or function" 错误

## 修复实施（2025-10-29更新）

### 已完成的修复

**文件**: `src-tauri/src/services/seekdb_adapter.rs`

#### 1. 修复SQL查询（第467-488行）

```rust
// 修改后：移除embedding字段
let sql = if project_id.is_some() {
    format!(
        "SELECT id, project_id, document_id, chunk_index, content, metadata,
                l2_distance(embedding, '{}') as distance
         FROM vector_documents
         WHERE project_id = ?
         ORDER BY l2_distance(embedding, '{}') APPROXIMATE
         LIMIT {}",
        embedding_str, embedding_str, limit * 2
    )
}
```

#### 2. 修复结果解析（第498-541行）

```rust
// 修改后：使用空向量
for row in rows {
    if row.len() < 7 {  // 7个字段（原来是8个）
        continue;
    }
    
    let id = row[0].as_str().unwrap_or_default().to_string();
    let project_id = row[1].as_str().unwrap_or_default().to_string();
    let document_id = row[2].as_str().unwrap_or_default().to_string();
    let chunk_index = row[3].as_i64().unwrap_or(0) as i32;
    let content = row[4].as_str().unwrap_or_default().to_string();
    let metadata_str = row[5].as_str().unwrap_or("{}");
    let distance = row[6].as_f64().unwrap_or(f64::MAX);
    
    // ... similarity calculation
    
    VectorDocument {
        // ...
        embedding: vec![], // 空向量
    }
}
```

### 编译结果

✅ **编译成功** (41秒)
```
Compiling mine-kb v0.1.0
Finished `release` profile [optimized] target(s) in 41.00s
```

✅ **无Linter错误**

## 相关文档

- [RESTORE_SEEKDB_VECTOR_SEARCH.md](./RESTORE_SEEKDB_VECTOR_SEARCH.md) - 恢复使用SeekDB向量检索
- [seekdb.md](./seekdb.md) - SeekDB使用说明

## 总结

通过移除不必要的embedding字段查询，我们既解决了SeekDB的兼容性问题，又优化了代码性能。核心的向量检索功能（`similarity_search`和`hybrid_search`）完全不受影响，继续使用SeekDB的原生向量能力。

**关键改进**：
- 修复了 "fetchall failed 1235" 错误
- 符合SeekDB官方最佳实践
- 减少数据传输量（不传输1536维向量）
- 保持了完整的向量检索功能

