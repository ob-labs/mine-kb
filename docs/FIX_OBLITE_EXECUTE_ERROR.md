# 修复 ObLite execute() 参数化查询错误

> **历史文档**: 本文档记录了早期版本（0.0.1.dev2）使用 `oblite` 模块时的问题。  
> **当前版本**: SeekDB 0.0.1.dev4 版本已更改模块名为 `seekdb`，但该限制仍然存在。  
> **参考**: [SeekDB 0.0.1.dev4 升级指南](UPGRADE_SEEKDB_0.0.1.dev4.md)

## 问题描述

创建知识库时出现以下错误：

```
Error：创建知识库失败：创建项目失败：Python subprocess error: ExecuteError - 
execute: incompatible function arguments. The following argument types are supported:
1. (self: oblite.ObLiteiEmbedCursor, arg0: str) -> int
Invoked with: <oblite.ObLiteiEmbedCursor object at 0xffffa1789170>,
'INSERT INTO projects (id, name, description, status, document_count, created_at, updated_at)
 VALUES (?, ?, ?, ?, ?, ?, ?) 
 ON DUPLICATE KEY UPDATE ...',
['3600f13e-51df-4616-8de6-6aa4fa9e904b', 'Untitled', '', 'Created', 0, 
 '2025-10-29T05:42:49.609341126+00:00', '2025-10-29T05:42:49.609341126+00:00']
```

## 根本原因

**ObLite 的 `cursor.execute()` 方法只接受一个字符串参数，不支持参数化查询。**

之前的实现尝试使用类似 SQLite/MySQL 的参数化查询方式：
```python
cursor.execute(sql, values)  # ❌ ObLite 不支持
```

但 ObLite 只支持：
```python
cursor.execute(sql)  # ✅ 只接受一个字符串参数
```

## 解决方案

### 1. 修改 `seekdb_bridge.py`

在 `SeekDBBridge` 类中添加了两个辅助方法：

#### 1.1 `format_sql_value()` - 格式化 Python 值为 SQL 字符串

```python
def format_sql_value(self, value: Any) -> str:
    """Format a Python value to SQL string representation for ObLite"""
    if value is None:
        return "NULL"
    elif isinstance(value, bool):
        return "1" if value else "0"
    elif isinstance(value, (int, float)):
        return str(value)
    elif isinstance(value, str):
        # Escape single quotes in strings
        escaped = value.replace("'", "''")
        return f"'{escaped}'"
    elif isinstance(value, list):
        # For vector/array values
        return str(value)
    else:
        # For other types, convert to string and quote
        escaped = str(value).replace("'", "''")
        return f"'{escaped}'"
```

#### 1.2 `build_sql_with_values()` - 将参数嵌入到 SQL 字符串中

```python
def build_sql_with_values(self, sql: str, values: List[Any]) -> str:
    """
    Replace ? placeholders in SQL with actual values
    ObLite doesn't support parameterized queries, so we embed values directly
    """
    if not values:
        return sql
    
    # Replace ? with actual values
    result = sql
    for value in values:
        formatted_value = self.format_sql_value(value)
        # Replace the first occurrence of ?
        result = result.replace("?", formatted_value, 1)
    
    return result
```

### 2. 更新三个处理方法

#### 2.1 `handle_execute()`

**修改前：**
```python
def handle_execute(self, params: Dict[str, Any]):
    try:
        sql = params["sql"]
        values = params.get("values", [])
        
        if values:
            self.cursor.execute(sql, values)  # ❌ 两个参数
        else:
            self.cursor.execute(sql)
```

**修改后：**
```python
def handle_execute(self, params: Dict[str, Any]):
    try:
        sql = params["sql"]
        values = params.get("values", [])
        
        # ObLite doesn't support parameterized queries, embed values directly
        final_sql = self.build_sql_with_values(sql, values)
        
        # ObLite execute() only accepts one argument
        self.cursor.execute(final_sql)  # ✅ 一个参数
```

#### 2.2 `handle_query()` 和 `handle_query_one()`

同样的修改应用到查询方法：
```python
# 嵌入参数值
final_sql = self.build_sql_with_values(sql, values)

# 只使用一个参数调用 execute
self.cursor.execute(final_sql)
```

## 关于 `ON DUPLICATE KEY UPDATE` 语法

ObLite 基于 OceanBase，支持 MySQL 兼容语法，因此 `ON DUPLICATE KEY UPDATE` 是被支持的。

原始 SQL：
```sql
INSERT INTO projects (id, name, description, status, document_count, created_at, updated_at)
VALUES (?, ?, ?, ?, ?, ?, ?)
ON DUPLICATE KEY UPDATE
    name = VALUES(name),
    description = VALUES(description),
    status = VALUES(status),
    document_count = VALUES(document_count),
    updated_at = VALUES(updated_at)
```

修复后会被转换为：
```sql
INSERT INTO projects (id, name, description, status, document_count, created_at, updated_at)
VALUES ('3600f13e-51df-4616-8de6-6aa4fa9e904b', 'Untitled', '', 'Created', 0, 
        '2025-10-29T05:42:49.609341126+00:00', '2025-10-29T05:42:49.609341126+00:00')
ON DUPLICATE KEY UPDATE
    name = VALUES(name),
    description = VALUES(description),
    status = VALUES(status),
    document_count = VALUES(document_count),
    updated_at = VALUES(updated_at)
```

## 测试验证

创建了测试脚本 `scripts/test_oblite_upsert.py` 用于验证不同的 UPSERT 语法支持。

## 修改文件

- `src-tauri/python/seekdb_bridge.py` - Python 桥接层主要修复

## 影响范围

此修复影响所有通过 Python 子进程与 ObLite 交互的数据库操作：
- 项目创建和更新
- 文档插入
- 向量数据存储
- 对话记录保存
- 所有 SQL 查询

## 测试建议

1. **创建新项目**：测试项目创建功能
2. **更新项目**：测试项目更新功能（应触发 UPSERT）
3. **添加文档**：测试文档添加和向量存储
4. **搜索功能**：测试向量相似度搜索
5. **对话功能**：测试对话记录的保存和加载

## 相关文档

- `docs/seekdb.md` - SeekDB/ObLite 使用文档
- `scripts/test_oblite_upsert.py` - UPSERT 语法测试脚本

## 修复日期

2025-10-29

## 修复人员

AI Assistant (Cursor)

