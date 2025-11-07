# SeekDB 使用方法与实践经验总结

**文档版本**: 2.0  
**最后更新**: 2025-11-05  
**适用版本**: SeekDB 0.0.1.dev4  
**作者**: MineKB Team

> **重要更新**: 本文档已更新至 SeekDB 0.0.1.dev4 版本，主要变更：
> - 模块名称从 `oblite` 更改为 `seekdb`
> - 新增向量列类型输出支持
> - 新增数据库存在性验证
> - 新增 USE 语句稳定支持
> - 新增自动提交模式（autocommit 参数）

---

## 📋 目录

1. [SeekDB 简介](#1-seekdb-简介)
2. [安装与配置](#2-安装与配置)
3. [基本使用](#3-基本使用)
4. [核心功能详解](#4-核心功能详解)
5. [MineKB 项目实践](#5-minekb-项目实践)
6. [常见问题与解决方案](#6-常见问题与解决方案)
7. [最佳实践](#7-最佳实践)
8. [注意事项与限制](#8-注意事项与限制)
9. [性能优化建议](#9-性能优化建议)
10. [总结与展望](#10-总结与展望)

---

## 1. SeekDB 简介

### 1.1 什么是 SeekDB？

SeekDB（基于 OceanBase Lite）是一款轻量级嵌入式数据库，以库的形式集成在应用程序中，为开发者提供 **ALL IN ONE** 的数据管理能力：

- **TP (OLTP)**: 事务处理能力
- **AP (OLAP)**: 分析查询能力  
- **AI Native**: 原生 AI 能力（向量检索、全文检索、混合检索）

### 1.2 核心特性

| 特性分类 | 功能 | 说明 |
|---------|------|------|
| **AI Native** | 向量检索 | HNSW 索引，支持近似最近邻搜索 |
| | 全文检索 | 内置 Fulltext Index |
| | 混合检索 | 语义搜索 + 关键词搜索 |
| **OLAP** | 列存 | Column Group 支持 |
| | 数据导入 | 旁路导入（Direct Load） |
| | 物化视图 | 自动刷新的 Materialized View |
| | 外表 | 直接查询 CSV 等外部文件 |
| **OLTP** | 事务 | ACID 事务支持 |
| | 索引 | B-tree、Vector Index |
| **部署** | 嵌入式 | 无需单独部署服务 |
| | 轻量级 | 适用于边缘计算、IoT、移动应用 |

### 1.3 适用场景

✅ **适合使用 SeekDB 的场景**：
- 嵌入式 AI 应用（向量检索）
- 知识库、文档搜索系统
- 边缘计算、IoT 设备
- 单机应用需要分析能力
- 快速原型验证

❌ **不适合使用 SeekDB 的场景**：
- 大规模分布式系统
- 需要高并发写入（千级 TPS 以上）
- 跨机器的分布式事务
- 需要复杂的数据库管理功能

---

## 2. 安装与配置

### 2.1 安装方式

#### 方式一：通过 pip 安装（推荐）

```bash
# 使用清华镜像源安装最新版本
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/

# 验证安装（注意：0.0.1.dev4 版本使用 seekdb 模块）
python3 -c "import seekdb; print('SeekDB 0.0.1.dev4 安装成功')"
```

#### 方式二：自动安装（MineKB 应用）

MineKB 应用启动时会自动：
1. 创建 Python 虚拟环境（`<app_data_dir>/venv/`）
2. 安装 seekdb 包
3. 验证安装成功

**应用数据目录位置**：
- **macOS**: `~/Library/Application Support/com.mine-kb.app/`
- **Linux**: `~/.local/share/com.mine-kb.app/`
- **Windows**: `%APPDATA%\com.mine-kb.app\`

#### 方式三：手动下载（不推荐）

```bash
# 注意：0.0.1.dev4 版本建议通过 pip 安装
# 如需手动安装，请参考官方文档
# 不再推荐直接下载 .so 文件的方式
```

### 2.2 基本配置

```json
// config.json
{
  "database": {
    "path": "./mine_kb.db",      // 数据库实例路径（推荐使用清晰的名称）
    "name": "mine_kb"            // 数据库名称
  }
}
```

### 2.3 系统要求

| 组件 | 要求 |
|------|------|
| Python | 3.8+ |
| 操作系统 | Linux (Ubuntu 18.04+), macOS (10.15+), Windows 10+ |
| 架构 | x86-64, ARM64 (pip 自动适配) |
| 内存 | 最低 2GB，推荐 4GB+ |
| 磁盘 | 最低 1GB 可用空间 |

---

## 3. 基本使用

### 3.1 快速开始

```python
import seekdb

# 1. 打开数据库实例
seekdb.open("./mine_kb.db")

# 2. 连接数据库
conn = seekdb.connect("test")
cursor = conn.cursor()

# 3. 创建表
cursor.execute("""
    CREATE TABLE t1 (
        c1 INT PRIMARY KEY,
        c2 INT
    )
""")

# 4. 插入数据
cursor.execute("INSERT INTO t1 VALUES(1, 10)")
cursor.execute("INSERT INTO t1 VALUES(2, 20)")

# 5. 提交事务
conn.commit()

# 6. 查询数据
cursor.execute("SELECT * FROM t1")
rows = cursor.fetchall()
print(rows)  # [(1, 10), (2, 20)]
```

### 3.2 数据库初始化流程

⚠️ **重要**：SeekDB 不会自动创建数据库，必须显式创建！

**0.0.1.dev4 版本新增**：数据库存在性验证功能，连接不存在的数据库会报错。

```python
import seekdb

# 正确的初始化流程
seekdb.open("./mine_kb.db")

# 1. 连接空字符串以访问系统上下文
admin_conn = seekdb.connect("")
admin_cursor = admin_conn.cursor()

# 2. 创建数据库（幂等操作）
admin_cursor.execute("CREATE DATABASE IF NOT EXISTS `my_database`")
admin_conn.commit()
admin_conn.close()

# 3. 现在连接到新创建的数据库
conn = seekdb.connect("my_database")
cursor = conn.cursor()

# 4. 创建表（现在可以成功）
cursor.execute("CREATE TABLE ...")
```

**0.0.1.dev4 新特性：自动提交模式**

```python
# 自动提交模式（无需手动 commit）
conn = seekdb.connect(db_name='my_database', autocommit=True)
cursor = conn.cursor()
cursor.execute("INSERT INTO t1 VALUES(1, 10)")  # 自动提交
```

### 3.3 常用 SQL 操作

```python
# 创建表
cursor.execute("""
    CREATE TABLE IF NOT EXISTS users (
        id VARCHAR(36) PRIMARY KEY,
        name TEXT NOT NULL,
        age INT,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    )
""")

# 插入数据
cursor.execute(
    "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
    ("user1", "Alice", 30)
)

# 查询数据
cursor.execute("SELECT * FROM users WHERE age > 25")
rows = cursor.fetchall()

# 更新数据
cursor.execute("UPDATE users SET age = 31 WHERE id = 'user1'")

# 删除数据
cursor.execute("DELETE FROM users WHERE age < 18")

# 提交事务
conn.commit()
```

---

## 4. 核心功能详解

### 4.1 向量检索（Vector Search）

#### 4.1.1 创建向量表

```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()

# 创建带向量字段的表
cursor.execute("""
    CREATE TABLE test_vector (
        c1 INT PRIMARY KEY,
        c2 vector(2),
        VECTOR INDEX idx1(c2) WITH (
            distance=l2,
            type=hnsw,
            lib=vsag
        )
    )
""")
```

**向量索引参数说明**：
- `distance`: 距离度量方式
  - `l2`: 欧氏距离（常用）
  - `cosine`: 余弦相似度
  - `ip`: 内积
- `type`: 索引类型
  - `hnsw`: 分层可导航小世界图（推荐）
- `lib`: 底层库
  - `vsag`: 默认向量库

#### 4.1.2 插入向量数据

```python
# 插入向量
cursor.execute("INSERT INTO test_vector VALUES(1, [1.0, 1.0])")
cursor.execute("INSERT INTO test_vector VALUES(2, [1.0, 2.0])")
cursor.execute("INSERT INTO test_vector VALUES(3, [1.0, 3.0])")
conn.commit()
```

#### 4.1.3 向量检索查询

```python
# 近似最近邻搜索（推荐）
cursor.execute("""
    SELECT c1, l2_distance(c2, '[1.0, 2.5]') as distance
    FROM test_vector
    ORDER BY l2_distance(c2, '[1.0, 2.5]') APPROXIMATE
    LIMIT 2
""")

results = cursor.fetchall()
print(results)  # [(2, 0.5), (3, 0.5)]
```

**⚠️ 重要限制**（0.0.1.dev4 版本部分改进）：
- ⚠️ 在使用向量函数时 SELECT vector 字段可能有限制
- ✅ 推荐只 SELECT 主键、元数据和距离值
- ✅ **0.0.1.dev4 新增**：支持向量列类型输出（在某些场景下）

```python
# ❌ 不推荐的用法（可能在某些场景下失败）
cursor.execute("""
    SELECT c1, c2, l2_distance(c2, '[1.0, 2.5]') as distance
    FROM test_vector
    ORDER BY l2_distance(c2, '[1.0, 2.5]') APPROXIMATE
""")
# 可能报错：fetchall failed 1235 Not supported feature or function

# ✅ 推荐用法（稳定可靠）
cursor.execute("""
    SELECT c1, l2_distance(c2, '[1.0, 2.5]') as distance
    FROM test_vector
    ORDER BY l2_distance(c2, '[1.0, 2.5]') APPROXIMATE
""")

# ✅ 0.0.1.dev4 新增：单独查询向量列（不使用向量函数时）
cursor.execute("SELECT c1, c2 FROM test_vector WHERE c1 = 1")
```

### 4.2 全文检索（Full-text Search）

#### 4.2.1 创建全文索引表

```python
cursor.execute("""
    CREATE TABLE articles (
        title VARCHAR(200) PRIMARY KEY,
        body TEXT,
        FULLTEXT fts_idx(title, body)
    )
""")
```

#### 4.2.2 插入文档

```python
cursor.execute("""
    INSERT INTO articles(title, body) VALUES
        ('OceanBase Tutorial', 'This is a tutorial about OceanBase Fulltext.'),
        ('Fulltext Index', 'Fulltext index can be very useful.'),
        ('OceanBase Test Case', 'Writing test cases helps ensure quality.')
""")
conn.commit()
```

#### 4.2.3 全文搜索

```python
cursor.execute("""
    SELECT 
        title,
        MATCH(title, body) AGAINST("OceanBase") as score
    FROM articles
    WHERE MATCH(title, body) AGAINST("OceanBase")
    ORDER BY score DESC
""")

results = cursor.fetchall()
print(results)
# [('OceanBase Tutorial', score1), ('OceanBase Test Case', score2)]
```

### 4.3 混合检索（Hybrid Search）

混合检索结合了向量检索（语义理解）和关键词检索（精确匹配），提供更好的搜索效果。

⚠️ **注意**：混合检索功能需要 SeekDB patch44x 版本支持，当前轻量版暂不支持。

```python
# 创建混合检索表
cursor.execute("""
    CREATE TABLE doc_table (
        c1 INT,
        vector vector(3),
        query VARCHAR(255),
        content VARCHAR(255),
        VECTOR INDEX idx1(vector) WITH (distance=l2, type=hnsw, lib=vsag),
        FULLTEXT idx2(query),
        FULLTEXT idx3(content)
    )
""")

# 插入数据
cursor.execute("""
    INSERT INTO doc_table VALUES
        (1, '[1,2,3]', 'hello world', 'oceanbase Elasticsearch database'),
        (2, '[1,2,1]', 'hello world, what is your name', 'oceanbase mysql database'),
        (3, '[1,1,1]', 'hello world, how are you', 'oceanbase oracle database')
""")
conn.commit()

# 混合检索查询
cursor.execute("""
    SET @parm = '{
        "query": {
            "bool": {
                "must": [
                    {"match": {"query": "hi hello"}},
                    {"match": {"content": "oceanbase mysql"}}
                ]
            }
        },
        "knn": {
            "field": "vector",
            "k": 5,
            "num_candidates": 10,
            "query_vector": [1,2,3],
            "boost": 0.7
        },
        "_source": ["query", "content", "_keyword_score", "_semantic_score"]
    }'
""")

cursor.execute("SELECT dbms_hybrid_search.search('doc_table', @parm)")
results = cursor.fetchall()
```

### 4.4 OLAP 分析能力

#### 4.4.1 列存表

```python
# 创建列存表
cursor.execute("""
    CREATE TABLE each_column_group (
        col1 VARCHAR(30) NOT NULL,
        col2 VARCHAR(30) NOT NULL,
        col3 VARCHAR(30) NOT NULL,
        col4 VARCHAR(30) NOT NULL,
        col5 INT
    ) WITH COLUMN GROUP (EACH COLUMN)
""")

# 插入数据
cursor.execute("INSERT INTO each_column_group VALUES('a', 'b', 'c', 'd', 1)")
conn.commit()

# 列式查询（只读取需要的列，性能更优）
cursor.execute("SELECT col1, col2 FROM each_column_group")
```

#### 4.4.2 数据导入（Direct Load）

```python
# 快速导入大量数据
cursor.execute("""
    LOAD DATA /*+ direct(true, 0) */ 
    INFILE '/data/1/example.csv' 
    INTO TABLE test_olap 
    FIELDS TERMINATED BY ','
""")
```

#### 4.4.3 外表查询

```python
# 创建外表（无需导入数据，直接查询文件）
cursor.execute("""
    CREATE EXTERNAL TABLE test_external_table (
        c1 INT,
        c2 INT
    ) 
    LOCATION='/data/1'
    FORMAT=(TYPE='CSV' FIELD_DELIMITER=',')
    PATTERN='example.csv'
""")

# 直接查询外部文件
cursor.execute("SELECT * FROM test_external_table")
```

### 4.5 OLTP 事务能力

```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()

# 创建表
cursor.execute("""
    CREATE TABLE test_oltp (
        c1 INT PRIMARY KEY,
        c2 INT
    )
""")

# 事务操作
try:
    cursor.execute("INSERT INTO test_oltp VALUES(1, 10)")
    cursor.execute("INSERT INTO test_oltp VALUES(2, 20)")
    cursor.execute("INSERT INTO test_oltp VALUES(3, 30)")
    
    # 提交事务
    conn.commit()
    
except Exception as e:
    # 回滚事务
    conn.rollback()
    print(f"Transaction failed: {e}")

# 查询（包含事务版本号）
cursor.execute("SELECT *, ORA_ROWSCN FROM test_oltp")
print(cursor.fetchall())

# 0.0.1.dev4 新增：自动提交模式
conn_auto = seekdb.connect("test", autocommit=True)
cursor_auto = conn_auto.cursor()
cursor_auto.execute("INSERT INTO test_oltp VALUES(4, 40)")  # 自动提交
```

---

## 5. MineKB 项目实践

### 5.1 架构设计

MineKB 使用 **Rust + Python + SeekDB** 的架构：

```
┌─────────────────┐
│   Rust (Tauri)  │  ← 主应用程序
│   Frontend      │
└────────┬────────┘
         │ JSON Protocol (stdin/stdout)
         ▼
┌─────────────────┐
│  Python Bridge  │  ← seekdb_bridge.py
│  (subprocess)   │
└────────┬────────┘
         │ Python API
         ▼
┌─────────────────┐
│     SeekDB      │  ← oblite.db/
│  (oblite.so)    │
└─────────────────┘
```

**优势**：
- Rust 提供高性能前端和业务逻辑
- Python 作为 SeekDB 的官方接口语言
- JSON 协议简单、可靠、易于调试

### 5.2 数据库设计

#### 5.2.1 表结构

```sql
-- 项目表
CREATE TABLE projects (
    id VARCHAR(36) PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    document_count INTEGER DEFAULT 0,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- 向量文档表（核心）
CREATE TABLE vector_documents (
    id VARCHAR(36) PRIMARY KEY,
    project_id VARCHAR(36) NOT NULL,
    document_id VARCHAR(36) NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536),          -- DashScope text-embedding-v1
    metadata TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(document_id, chunk_index)
);

-- 向量索引
CREATE VECTOR INDEX idx_embedding 
ON vector_documents(embedding) 
WITH (distance=l2, type=hnsw, lib=vsag);

-- 普通索引
CREATE INDEX idx_project_id ON vector_documents(project_id);
CREATE INDEX idx_document_id ON vector_documents(document_id);

-- 会话表
CREATE TABLE conversations (
    id VARCHAR(36) PRIMARY KEY,
    project_id VARCHAR(36) NOT NULL,
    title TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    message_count INTEGER DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- 消息表
CREATE TABLE messages (
    id VARCHAR(36) PRIMARY KEY,
    conversation_id VARCHAR(36) NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    sources TEXT,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);
```

#### 5.2.2 索引策略

| 表 | 索引类型 | 字段 | 用途 |
|----|---------|------|------|
| vector_documents | VECTOR INDEX | embedding | 向量检索 |
| vector_documents | B-tree | project_id | 项目过滤 |
| vector_documents | B-tree | document_id | 文档查询 |
| conversations | B-tree | project_id | 项目会话列表 |
| messages | B-tree | conversation_id | 消息历史 |

### 5.3 Python Bridge 实现

#### 5.3.1 JSON 协议

**命令格式**（stdin → Python）：
```json
{
  "command": "init",
  "params": {
    "db_path": "./oblite.db",
    "db_name": "mine_kb"
  }
}
```

**响应格式**（Python → stdout）：
```json
{
  "status": "success",
  "data": {"db_path": "./oblite.db", "db_name": "mine_kb"}
}
```

或错误响应：
```json
{
  "status": "error",
  "error": "InitError",
  "details": "数据库初始化失败..."
}
```

#### 5.3.2 支持的命令

| 命令 | 说明 | 参数 |
|-----|------|------|
| `init` | 初始化数据库连接 | db_path, db_name |
| `execute` | 执行 SQL（INSERT/UPDATE/DELETE） | sql, values |
| `query` | 查询数据（SELECT） | sql, values |
| `query_one` | 查询单行数据 | sql, values |
| `commit` | 提交事务 | - |
| `rollback` | 回滚事务 | - |
| `ping` | 健康检查 | - |

#### 5.3.3 关键实现细节

**参数化查询处理**：

SeekDB 不支持标准的参数化查询（`?` 占位符），需要手动替换：

```python
def format_sql_value(self, value: Any) -> str:
    """将 Python 值转换为 SQL 字符串表示"""
    if value is None:
        return "NULL"
    elif isinstance(value, bool):
        return "1" if value else "0"
    elif isinstance(value, (int, float)):
        return str(value)
    elif isinstance(value, str):
        # 转义单引号
        escaped = value.replace("'", "''")
        return f"'{escaped}'"
    elif isinstance(value, list):
        # 向量/数组值
        return str(value)
    else:
        escaped = str(value).replace("'", "''")
        return f"'{escaped}'"

def build_sql_with_values(self, sql: str, values: List[Any]) -> str:
    """替换 SQL 中的 ? 占位符为实际值"""
    if not values:
        return sql
    
    result = sql
    for value in values:
        formatted_value = self.format_sql_value(value)
        result = result.replace("?", formatted_value, 1)
    
    return result
```

**类型转换**：

```python
def convert_value_for_json(self, value: Any) -> Any:
    """将 Python 对象转换为 JSON 可序列化格式"""
    if value is None:
        return None
    elif isinstance(value, (datetime, date)):
        # datetime → ISO 字符串
        return value.isoformat()
    elif isinstance(value, Decimal):
        # Decimal → float
        return float(value)
    elif isinstance(value, bytes):
        # bytes → base64 字符串
        import base64
        return base64.b64encode(value).decode('utf-8')
    elif isinstance(value, (list, tuple)):
        return [self.convert_value_for_json(v) for v in value]
    else:
        return value
```

### 5.4 向量检索实现

#### 5.4.1 检索流程

```
用户查询
   ↓
生成 query embedding (DashScope API)
   ↓
向量检索 SQL (l2_distance + APPROXIMATE)
   ↓
计算相似度分数 (1 - distance/sqrt(2))
   ↓
过滤低分结果 (threshold=0.3)
   ↓
返回相关文档块
```

#### 5.4.2 SQL 实现

```sql
SELECT 
    id, 
    project_id, 
    document_id, 
    chunk_index, 
    content, 
    metadata,
    l2_distance(embedding, '[...]') as distance
FROM vector_documents
WHERE project_id = ?
ORDER BY l2_distance(embedding, '[...]') APPROXIMATE
LIMIT 20
```

**关键点**：
- 不 SELECT `embedding` 字段（避免 1235 错误）
- 使用 `APPROXIMATE` 关键字（HNSW 近似搜索）
- 限制返回数量（`LIMIT 20`）
- 在应用层计算相似度并过滤

#### 5.4.3 相似度计算

```rust
// L2 距离 → 相似度分数
// 假设向量已归一化，最大距离约为 sqrt(2)
let similarity = 1.0 - (distance / std::f64::consts::SQRT_2);

// 过滤低分结果
if similarity >= 0.3 {
    results.push(doc);
}
```

### 5.5 数据迁移

从 SQLite 迁移到 SeekDB：

```bash
python migrate_sqlite_to_seekdb.py <sqlite_path> <seekdb_path>
```

**迁移内容**：
- ✅ 所有项目元数据
- ✅ 所有会话和消息历史
- ✅ 所有文档块和向量 embeddings
- ✅ 自动创建 HNSW 索引
- ✅ 数据完整性验证

**注意事项**：
- Embedding 维度统一为 1536（DashScope 标准）
- SQLite BLOB → JSON 数组字符串
- 时间戳格式转换

---

## 6. 常见问题与解决方案

### 6.1 安装问题

#### 问题 1: ModuleNotFoundError: No module named 'seekdb'

**原因**：seekdb 包未安装或版本不正确

**解决方案**：
```bash
# 方案 1: 安装最新版本
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/

# 方案 2: 检查虚拟环境
source ~/.local/share/com.mine-kb.app/venv/bin/activate
pip list | grep seekdb

# 方案 3: 验证导入（注意：0.0.1.dev4 使用 seekdb 模块）
python -c "import seekdb; print('SeekDB 0.0.1.dev4 OK')"
```

> **注意**：0.0.1.dev4 版本应使用 `import seekdb`，不再使用 `import oblite`。

#### 问题 2: pip install 失败（No such file or directory）

**原因**：直接使用 `pip` 命令，但虚拟环境中可能没有 `pip` 可执行文件

**解决方案**：
```bash
# 使用 python -m pip 替代
python -m pip install seekdb==0.0.1.dev2 -i https://pypi.tuna.tsinghua.edu.cn/simple/
```

### 6.2 数据库初始化问题

#### 问题 3: Unknown database (错误码 1049)

**原因**：SeekDB 不会自动创建数据库。**0.0.1.dev4 新增**：数据库存在性验证，连接不存在的数据库会报错。

**解决方案**：
```python
# 显式创建数据库
admin_conn = seekdb.connect("")
admin_cursor = admin_conn.cursor()
admin_cursor.execute("CREATE DATABASE IF NOT EXISTS `mine_kb`")
admin_conn.commit()
admin_conn.close()

# 然后连接
conn = seekdb.connect("mine_kb")
```

#### 问题 4: 应用启动后数据库连接失败

**诊断步骤**：
```bash
# 1. 检查数据库目录
ls -la ~/.local/share/com.mine-kb.app/mine_kb.db/

# 2. 查看日志
cat ~/.local/share/com.mine-kb.app/mine_kb.db/log/oblite.log

# 3. 手动测试连接（使用 seekdb 模块）
python3 <<EOF
import seekdb
seekdb.open("~/.local/share/com.mine-kb.app/mine_kb.db")
conn = seekdb.connect("")
cursor = conn.cursor()
cursor.execute("CREATE DATABASE IF NOT EXISTS test")
print("OK")
EOF
```

### 6.3 查询错误

#### 问题 5: fetchall failed 1235 Not supported feature or function

**原因**：在使用向量函数时 SELECT 了 vector 字段

**错误示例**：
```sql
-- ❌ 错误
SELECT embedding, l2_distance(embedding, '[...]') as distance
FROM vector_documents
ORDER BY l2_distance(embedding, '[...]') APPROXIMATE
```

**解决方案**：
```sql
-- ✅ 正确
SELECT id, content, l2_distance(embedding, '[...]') as distance
FROM vector_documents
ORDER BY l2_distance(embedding, '[...]') APPROXIMATE
```

#### 问题 6: ORDER BY 不支持（错误码 1235）

**原因**：SeekDB 不支持普通的 `ORDER BY`（除了向量检索专用的 `ORDER BY l2_distance(...) APPROXIMATE`）

**解决方案**：
```python
# 在应用层排序
rows = subprocess.query(
    "SELECT id, name, updated_at FROM projects",
    []
)

# 在内存中排序
rows.sort(key=lambda row: row[2], reverse=True)  # 按 updated_at DESC
```

#### 问题 7: USE database 语句失败

**原因**：旧版本 SeekDB 的 `USE` 语句行为不稳定

**解决方案**（0.0.1.dev4 已改进）：
```python
# ✅ 推荐方式：直接通过 connect() 指定数据库
conn = seekdb.connect("mine_kb")

# ✅ 0.0.1.dev4 支持：USE 语句（已稳定）
cursor.execute("USE mine_kb")
```

> **注意**：0.0.1.dev4 版本已稳定支持 USE 语句，但仍推荐使用 `connect(db_name)` 方式。

### 6.4 性能问题

#### 问题 8: 向量检索速度慢

**诊断**：
```python
import time

start = time.time()
cursor.execute("""
    SELECT id FROM vector_documents
    ORDER BY l2_distance(embedding, '[...]') APPROXIMATE
    LIMIT 10
""")
results = cursor.fetchall()
elapsed = time.time() - start
print(f"Query time: {elapsed:.3f}s")
```

**优化方案**：
1. 确保使用了 `APPROXIMATE` 关键字
2. 减少 `LIMIT` 数量
3. 添加 `WHERE` 过滤条件（如 `project_id`）
4. 检查 HNSW 索引是否创建成功

```sql
-- 查看索引
SHOW INDEX FROM vector_documents;
```

#### 问题 9: 查询返回数据量大导致内存占用高

**解决方案**：
```python
# 1. 分页查询
def query_in_batches(cursor, sql, batch_size=1000):
    offset = 0
    while True:
        batch_sql = f"{sql} LIMIT {batch_size} OFFSET {offset}"
        cursor.execute(batch_sql)
        rows = cursor.fetchall()
        
        if not rows:
            break
        
        yield from rows
        offset += batch_size

# 2. 只查询需要的字段（不要 SELECT *）
cursor.execute("SELECT id, content FROM vector_documents WHERE ...")

# 3. 不查询 vector 字段
# ❌ SELECT embedding FROM ...
# ✅ 只查询元数据
```

### 6.5 数据一致性问题

#### 问题 10: 事务未提交导致数据丢失

**原因**：忘记调用 `conn.commit()`

**解决方案**：
```python
try:
    cursor.execute("INSERT INTO ...")
    cursor.execute("UPDATE ...")
    conn.commit()  # ✅ 必须调用
except Exception as e:
    conn.rollback()
    raise
```

#### 问题 11: 外键约束失败

**诊断**：
```sql
-- 检查外键约束
SELECT * FROM information_schema.table_constraints 
WHERE constraint_type = 'FOREIGN KEY';
```

**解决方案**：
```python
# 确保按正确顺序插入（先父表，后子表）
cursor.execute("INSERT INTO projects ...")
conn.commit()

cursor.execute("INSERT INTO conversations ...")  # 引用 projects
conn.commit()
```

---

## 7. 最佳实践

### 7.1 表设计

#### ✅ 推荐做法

```sql
-- 1. 使用合适的主键类型
CREATE TABLE projects (
    id VARCHAR(36) PRIMARY KEY,  -- UUID
    name TEXT NOT NULL,
    created_at DATETIME NOT NULL
);

-- 2. 为常用查询字段创建索引
CREATE INDEX idx_project_name ON projects(name);
CREATE INDEX idx_created_at ON projects(created_at);

-- 3. 使用外键约束确保数据一致性
CREATE TABLE documents (
    id VARCHAR(36) PRIMARY KEY,
    project_id VARCHAR(36) NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- 4. 为向量字段创建 HNSW 索引
CREATE TABLE embeddings (
    id VARCHAR(36) PRIMARY KEY,
    content TEXT,
    embedding vector(1536),
    VECTOR INDEX idx_embedding(embedding) 
    WITH (distance=l2, type=hnsw, lib=vsag)
);
```

#### ❌ 避免做法

```sql
-- ❌ 不要使用 SELECT *
SELECT * FROM large_table;  -- 浪费带宽和内存

-- ❌ 不要在热点字段上使用 TEXT
CREATE TABLE users (
    email TEXT PRIMARY KEY  -- ❌ 应该用 VARCHAR(255)
);

-- ❌ 不要创建过多索引
CREATE INDEX idx1 ON table(col1);
CREATE INDEX idx2 ON table(col2);
CREATE INDEX idx3 ON table(col3);
-- ... 过多索引影响写入性能

-- ❌ 不要在 vector 字段上创建普通索引
CREATE INDEX idx_embedding ON embeddings(embedding);  -- ❌ 应该用 VECTOR INDEX
```

### 7.2 查询优化

#### ✅ 推荐做法

```sql
-- 1. 只查询需要的字段
SELECT id, name, created_at FROM projects WHERE status = 'active';

-- 2. 使用索引字段进行过滤
SELECT * FROM vector_documents WHERE project_id = '...';  -- 有索引

-- 3. 向量检索使用 APPROXIMATE
SELECT id, l2_distance(embedding, '[...]') as distance
FROM vector_documents
ORDER BY l2_distance(embedding, '[...]') APPROXIMATE  -- ✅
LIMIT 10;

-- 4. 限制返回数量
SELECT * FROM messages ORDER BY created_at DESC LIMIT 100;
```

#### ❌ 避免做法

```sql
-- ❌ 不要在向量查询中 SELECT vector 字段
SELECT embedding, l2_distance(embedding, '[...]') as distance
FROM vector_documents;

-- ❌ 不要使用精确搜索（慢）
SELECT * FROM vector_documents
ORDER BY l2_distance(embedding, '[...]')  -- 缺少 APPROXIMATE
LIMIT 10;

-- ❌ 不要在非索引字段上过滤大表
SELECT * FROM large_table WHERE non_indexed_column = '...';

-- ❌ 不要使用 ORDER BY（除了向量检索）
SELECT * FROM projects ORDER BY name;  -- 不支持
```

### 7.3 事务管理

#### ✅ 推荐做法

```python
# 1. 使用上下文管理器（如果支持）
try:
    cursor.execute("INSERT ...")
    cursor.execute("UPDATE ...")
    conn.commit()
except Exception as e:
    conn.rollback()
    log.error(f"Transaction failed: {e}")
    raise

# 2. 批量操作在一个事务中
cursor.execute("BEGIN")
for item in items:
    cursor.execute("INSERT INTO ...", item)
conn.commit()

# 3. 读操作不需要事务
cursor.execute("SELECT * FROM projects")
rows = cursor.fetchall()
```

#### ❌ 避免做法

```python
# ❌ 不要忘记提交
cursor.execute("INSERT ...")
# 缺少 conn.commit()

# ❌ 不要在循环中提交（性能差）
for item in items:
    cursor.execute("INSERT ...")
    conn.commit()  # ❌ 每次都提交

# ❌ 不要嵌套事务（不支持）
conn.begin()
cursor.execute("INSERT ...")
conn.begin()  # ❌ 不支持
```

### 7.4 向量检索

#### ✅ 推荐做法

```python
# 1. 归一化查询向量
import numpy as np
query_embedding = np.array(embedding)
query_embedding = query_embedding / np.linalg.norm(query_embedding)

# 2. 使用阈值过滤结果
threshold = 0.3
results = [doc for doc in search_results if doc.similarity >= threshold]

# 3. 添加项目过滤条件
sql = """
    SELECT id, content, l2_distance(embedding, '{}') as distance
    FROM vector_documents
    WHERE project_id = ?
    ORDER BY l2_distance(embedding, '{}') APPROXIMATE
    LIMIT 20
"""

# 4. 返回元数据而非原始向量
# embedding 字段设为空向量
VectorDocument {
    id, content, metadata,
    embedding: vec![],  # 不返回原始向量
}
```

#### ❌ 避免做法

```python
# ❌ 不要查询所有项目的文档（慢）
sql = "SELECT * FROM vector_documents ORDER BY l2_distance(...) APPROXIMATE"

# ❌ 不要返回过多结果
LIMIT 1000  # ❌ 太多

# ❌ 不要在向量检索后再过滤
# 应该在 WHERE 子句中过滤
```

### 7.5 错误处理

#### ✅ 推荐做法

```python
import logging

try:
    cursor.execute(sql, values)
    conn.commit()
except Exception as e:
    logging.error(f"Database error: {e}")
    logging.error(f"SQL: {sql}")
    logging.error(f"Values: {values}")
    conn.rollback()
    
    # 返回友好的错误信息
    if "1049" in str(e):
        raise DatabaseError("数据库不存在，请检查配置")
    elif "1235" in str(e):
        raise DatabaseError("查询不支持，请检查 SQL 语句")
    else:
        raise
```

#### ❌ 避免做法

```python
# ❌ 不要忽略错误
try:
    cursor.execute(sql)
except:
    pass  # ❌ 吞掉错误

# ❌ 不要暴露敏感信息
except Exception as e:
    return str(e)  # ❌ 可能包含敏感路径、密码等
```

### 7.6 连接管理

#### ✅ 推荐做法

```python
import seekdb

# 1. 使用连接池（长期运行的服务）
class SeekDBPool:
    def __init__(self, db_path, db_name):
        self.db_path = db_path
        self.db_name = db_name
        self.conn = None
    
    def get_connection(self):
        if self.conn is None:
            seekdb.open(self.db_path)
            self.conn = seekdb.connect(self.db_name)
        return self.conn
    
    def close(self):
        if self.conn:
            self.conn.close()
            self.conn = None

# 2. 重用连接
pool = SeekDBPool("./mine_kb.db", "mine_kb")
conn = pool.get_connection()
cursor = conn.cursor()

# 3. 程序退出时关闭
import atexit
atexit.register(pool.close)
```

#### ❌ 避免做法

```python
# ❌ 不要频繁创建/关闭连接
for query in queries:
    conn = seekdb.connect(db_name)  # ❌ 每次都连接
    cursor = conn.cursor()
    cursor.execute(query)
    conn.close()

# ❌ 不要忘记关闭连接（资源泄漏）
conn = seekdb.connect(db_name)
# ... 使用连接
# 缺少 conn.close()
```

---

## 8. 注意事项与限制

### 8.1 已知限制

| 限制 | 说明 | 解决方案 |
|-----|------|---------|
| **不支持 ORDER BY** | 除了向量检索的 `ORDER BY l2_distance(...) APPROXIMATE` | 在应用层排序 |
| **不支持 SELECT vector 字段** | 在使用向量函数时不能同时 SELECT vector 字段 | 使用空向量替代 |
| **不支持参数化查询** | `execute(sql, params)` 不支持 | 手动构建 SQL（注意防注入） |
| **不支持 USE 语句** | `USE database` 行为不稳定 | 通过 `connect(db_name)` 指定 |
| **不自动创建数据库** | `connect()` 不会创建数据库 | 显式 `CREATE DATABASE` |
| **混合检索未实装** | 轻量版暂不支持混合检索 | 等待 patch44x 版本 |
| **物化视图有限** | 功能尚不完整 | 谨慎使用 |
| **并发写入受限** | 单机数据库，写入性能有限 | 批量写入，避免频繁提交 |

### 8.2 性能限制

| 指标 | 典型值 | 说明 |
|-----|-------|------|
| 向量检索延迟 | 10-100ms | 取决于数据量和 LIMIT |
| 写入 TPS | 100-1000 | 单机性能 |
| 最大向量维度 | 无明确限制 | 推荐 ≤2048 |
| 最大数据库大小 | 无明确限制 | 取决于磁盘空间 |
| 并发连接数 | 建议 1-10 | 嵌入式数据库 |

### 8.3 兼容性

| 组件 | 兼容性 |
|-----|-------|
| Python 版本 | 3.8+ |
| 操作系统 | Linux, macOS, Windows |
| CPU 架构 | x86-64, ARM64 |
| SQL 方言 | MySQL/OceanBase 风格 |
| 字符编码 | UTF-8 |

---

## 9. 性能优化建议

### 9.1 向量检索优化

```python
# 1. 使用合适的 LIMIT
LIMIT 10  # ✅ 推荐
LIMIT 100  # ⚠️ 可能较慢
LIMIT 1000  # ❌ 不推荐

# 2. 添加过滤条件
WHERE project_id = '...'  # ✅ 减少搜索空间

# 3. 调整 HNSW 参数（创建索引时）
WITH (
    distance=l2,
    type=hnsw,
    lib=vsag,
    -- 可能支持的参数（需验证）
    -- M=16,                -- HNSW 图的连接数
    -- ef_construction=200  -- 构建时的搜索深度
)
```

### 9.2 批量操作优化

```python
# ❌ 逐条插入（慢）
for item in items:
    cursor.execute("INSERT INTO table VALUES (?)", item)
    conn.commit()

# ✅ 批量插入（快）
cursor.execute("BEGIN")
for item in items:
    cursor.execute("INSERT INTO table VALUES (?)", item)
conn.commit()

# ✅ 使用 LOAD DATA（最快，适用于大量数据）
cursor.execute("""
    LOAD DATA /*+ direct(true, 0) */
    INFILE 'data.csv'
    INTO TABLE table
    FIELDS TERMINATED BY ','
""")
```

### 9.3 查询优化

```python
# 1. 避免全表扫描
# ❌ 不要
cursor.execute("SELECT * FROM large_table WHERE non_indexed_col = '...'")

# ✅ 使用索引
cursor.execute("SELECT * FROM large_table WHERE indexed_col = '...'")

# 2. 只查询需要的字段
# ❌ 不要
cursor.execute("SELECT * FROM table")

# ✅ 只查询需要的
cursor.execute("SELECT id, name FROM table")

# 3. 使用 EXPLAIN 分析查询（如果支持）
cursor.execute("EXPLAIN SELECT ...")
print(cursor.fetchall())
```

### 9.4 索引策略

```sql
-- 1. 为常用查询字段创建索引
CREATE INDEX idx_project_id ON vector_documents(project_id);

-- 2. 组合索引（如果需要）
CREATE INDEX idx_project_doc ON vector_documents(project_id, document_id);

-- 3. 定期检查索引使用情况
SHOW INDEX FROM vector_documents;

-- 4. 删除不使用的索引
DROP INDEX unused_idx ON table;
```

### 9.5 内存优化

```python
# 1. 分页查询大结果集
def fetch_in_batches(cursor, sql, batch_size=1000):
    offset = 0
    while True:
        batch_sql = f"{sql} LIMIT {batch_size} OFFSET {offset}"
        cursor.execute(batch_sql)
        rows = cursor.fetchall()
        
        if not rows:
            break
        
        for row in rows:
            yield row
        
        offset += batch_size

# 2. 及时释放不需要的数据
results = cursor.fetchall()
process_results(results)
del results  # 释放内存

# 3. 不要在内存中缓存大量数据
# ❌ 不要
all_docs = cursor.execute("SELECT * FROM large_table").fetchall()

# ✅ 流式处理
for row in cursor.execute("SELECT * FROM large_table"):
    process_row(row)
```

---

## 10. 总结与展望

### 10.1 SeekDB 的优势

| 优势 | 说明 |
|-----|------|
| **All-in-One** | TP + AP + AI 能力集成 |
| **原生向量检索** | HNSW 索引，性能优秀 |
| **轻量级部署** | 嵌入式，无需独立服务 |
| **平滑升级** | 可升级到分布式 OceanBase |
| **开发友好** | Python API，易于集成 |

### 10.2 SeekDB 的不足

| 不足 | 影响 | 缓解方案 |
|-----|------|---------|
| ORDER BY 限制 | 需要应用层排序 | 影响较小 |
| Vector 字段限制 | 不能直接查询 | 使用空向量 |
| 单机性能 | 并发写入受限 | 批量操作 |
| 文档不完善 | 学习曲线陡峭 | 参考示例代码 |
| 功能未完整 | 部分特性未实装 | 等待新版本 |

### 10.3 适用场景总结

#### ✅ 非常适合

- 嵌入式 AI 应用（RAG、知识库）
- 桌面应用需要向量检索
- 原型验证、MVP 开发
- 边缘计算、IoT 设备
- 单机部署的中小型应用

#### ⚠️ 需要评估

- 高并发写入场景（考虑批量优化）
- 大数据量场景（数百万条记录以上）
- 复杂分析查询（部分功能受限）

#### ❌ 不适合

- 分布式系统（应该用 OceanBase 分布式版）
- 需要高可用、容灾的生产环境
- 极高性能要求（万级 TPS）

### 10.4 未来展望

SeekDB/OceanBase Lite 的发展方向：

1. **功能完善**
   - 混合检索（Hybrid Search）全面支持
   - 更丰富的 OLAP 功能
   - 完善的物化视图

2. **性能提升**
   - 更快的向量检索
   - 更好的并发支持
   - 更低的内存占用

3. **易用性改进**
   - 完善的文档和示例
   - 更友好的错误信息
   - 可视化管理工具

4. **生态建设**
   - 更多语言的 SDK
   - 与主流框架的集成
   - 云原生支持

### 10.5 推荐学习路径

1. **入门阶段**（1-2 天）
   - 安装 SeekDB
   - 学习基本 SQL 操作
   - 创建第一个向量检索应用

2. **进阶阶段**（1 周）
   - 深入理解向量检索原理
   - 掌握 HNSW 索引调优
   - 学习事务和并发控制

3. **高级阶段**（2-4 周）
   - 生产环境部署优化
   - 性能调优和监控
   - 与现有系统集成

4. **专家阶段**（持续）
   - 贡献社区和开源项目
   - 深入研究底层实现
   - 探索创新应用场景

---

## 附录

### A. 相关文档

- `docs/seekdb.md` - SeekDB 基础文档
- `docs/MIGRATION_SEEKDB.md` - SQLite 迁移指南
- `docs/SEEKDB_AUTO_INSTALL.md` - 自动安装说明
- `docs/FIX_SEEKDB_VECTOR_QUERY.md` - 向量查询修复
- `docs/FIX_SEEKDB_ORDER_BY.md` - ORDER BY 问题修复
- `docs/FIX_SEEKDB_DATABASE_ERROR.md` - 数据库初始化修复
- `docs/SEEKDB_VECTOR_FIELD_LIMITATION_ANALYSIS.md` - Vector 字段限制分析

### C. 示例代码

完整示例代码请参考：
- `src-tauri/python/seekdb_bridge.py` - Python Bridge 实现（已更新至 0.0.1.dev4）
- `src-tauri/python/migrate_sqlite_to_seekdb.py` - 数据迁移脚本
- `src-tauri/python/test_seekdb.py` - 测试示例（已更新至 0.0.1.dev4）
- `src-tauri/src/services/seekdb_adapter.rs` - Rust 适配器

### D. 版本升级指南

- [UPGRADE_SEEKDB_0.0.1.dev4.md](UPGRADE_SEEKDB_0.0.1.dev4.md) - 从 0.0.1.dev2 升级到 0.0.1.dev4 的详细指南

### E. 技术支持

遇到问题？

1. 查阅本文档的"常见问题"章节
2. 查看 [UPGRADE_SEEKDB_0.0.1.dev4.md](UPGRADE_SEEKDB_0.0.1.dev4.md) 升级指南
3. 查看 `docs/` 目录下的相关修复文档
4. 检查应用日志（`<db_path>/log/oblite.log`）
5. 在 GitHub 上提交 Issue
6. 参考 MineKB 项目的实现

---

**文档结束**

感谢阅读！希望这份文档能帮助你更好地理解和使用 SeekDB。

如有任何问题或建议，欢迎反馈。

---

**版权声明**：本文档基于 MineKB 项目的实践经验总结，遵循项目开源协议。

