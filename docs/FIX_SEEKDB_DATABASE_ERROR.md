# 修复 SeekDB "Unknown Database" 错误

> **历史文档**: 本文档记录了早期版本的数据库初始化问题。  
> **当前版本**: SeekDB 0.0.1.dev4 已增强数据库存在性验证。  
> **参考**: [SeekDB 0.0.1.dev4 升级指南](UPGRADE_SEEKDB_0.0.1.dev4.md)

## 问题描述

应用启动后出现以下错误：
```
[SeekDB Bridge] Execute error: execute sql failed 1049 Unknown database
thread 'main' panicked at src/main.rs:354:21:
Failed to initialize app state: Python subprocess error: ExecuteError - execute sql failed 1049 Unknown database
```

## 根本原因

### 问题分析

1. **SeekDB 的数据库不会自动创建**
   - `oblite.connect(db_name)` 即使数据库不存在也不会抛出异常
   - 连接"成功"返回，但数据库实际上不存在
   - 执行 SQL 时才会报错 "Unknown database" (错误码 1049)

2. **错误的初始化流程**
   ```python
   # 错误的方式
   oblite.open(db_path)
   conn = oblite.connect("my_database")  # 不会创建数据库！
   cursor = conn.cursor()
   cursor.execute("CREATE TABLE ...")  # 失败：Unknown database
   ```

3. **SeekDB 的数据库模型**
   - SeekDB 基于 OceanBase Lite
   - `oblite.db` 是一个数据库实例（目录），类似 MySQL server
   - 实例中可以有多个数据库（schemas）
   - 必须先创建数据库，才能在其中创建表

### 日志证据

从 `/home/ubuntu/.local/share/com.mine-kb.app/oblite.db/log/oblite.log` 可以看到：
```
[2025-10-29 03:40:33.166283] WDIAG [SERVER] execute (ob_embed_impl.cpp:300) 
[50615][][T0][YB42C0A84003-0000000000500005-0-0] [lt=1][errcode=-5154] 
execute sql failed(ret=-5154, ret="OB_ERR_BAD_DATABASE", sql="CREATE TABLE ...")
```

错误码 `OB_ERR_BAD_DATABASE` 明确指出数据库不存在。

## 解决方案

### 正确的初始化流程

```python
# 1. 打开数据库实例
oblite.open(db_path)

# 2. 连接到空字符串以访问系统/管理上下文
admin_conn = oblite.connect("")

# 3. 创建数据库
admin_cursor = admin_conn.cursor()
admin_cursor.execute(f"CREATE DATABASE IF NOT EXISTS `{db_name}`")
admin_conn.commit()
admin_conn.close()

# 4. 现在连接到新创建的数据库
conn = oblite.connect(db_name)
cursor = conn.cursor()

# 5. 创建表（现在可以成功）
cursor.execute("CREATE TABLE ...")
```

### 关键点

1. **使用空字符串连接**: `oblite.connect("")` 可以访问系统上下文来执行 CREATE DATABASE
2. **IF NOT EXISTS**: 使用 `CREATE DATABASE IF NOT EXISTS` 确保幂等性
3. **先创建后连接**: 必须先创建数据库，然后再连接

## 实现细节

### 修改文件

**`src-tauri/python/seekdb_bridge.py`** - 修改 `handle_init()` 方法：

```python
def handle_init(self, params: Dict[str, Any]):
    """Initialize SeekDB connection"""
    try:
        db_path = params.get("db_path", "./oblite.db")
        db_name = params.get("db_name", "mine_kb")
        
        self.log(f"Initializing SeekDB: path={db_path}, db={db_name}")
        
        # Open database instance
        oblite.open(db_path)
        
        # Always ensure database exists before connecting
        # Note: oblite.connect() doesn't throw exception even if database doesn't exist
        try:
            self.log(f"Ensuring database '{db_name}' exists...")
            # Connect with empty string to access admin/system context
            admin_conn = oblite.connect("")
            admin_cursor = admin_conn.cursor()
            admin_cursor.execute(f"CREATE DATABASE IF NOT EXISTS `{db_name}`")
            admin_conn.commit()
            admin_conn.close()
            self.log(f"✅ Database '{db_name}' is ready")
        except Exception as create_error:
            self.log(f"⚠️  Warning: Failed to create database: {create_error}")
            # Continue anyway, maybe database already exists
        
        # Now connect to the database
        self.conn = oblite.connect(db_name)
        self.log(f"✅ Connected to database '{db_name}'")
        
        self.cursor = self.conn.cursor()
        self.db_path = db_path
        self.db_name = db_name
        
        # Note: USE statement not needed, connection already bound to database
        
        self.log("SeekDB initialized successfully")
        self.send_success({"db_path": db_path, "db_name": db_name})
        
    except Exception as e:
        self.log(f"Init error: {e}")
        self.log(f"Traceback: {traceback.format_exc()}")
        error_details = (
            f"数据库初始化失败\n"
            f"路径: {params.get('db_path', './oblite.db')}\n"
            f"数据库名: {params.get('db_name', 'mine_kb')}\n"
            f"错误: {str(e)}"
        )
        self.send_error("InitError", error_details)
```

## 修复验证

修复后的正常启动日志：

```
[SeekDB Bridge] SeekDB Bridge started, waiting for commands...
[SeekDB Bridge] Initializing SeekDB: path=/home/ubuntu/.local/share/com.mine-kb.app/oblite.db, db=mine-kb
[SeekDB Bridge] Ensuring database 'mine-kb' exists...
[SeekDB Bridge] ✅ Database 'mine-kb' is ready
[SeekDB Bridge] ✅ Connected to database 'mine-kb'
[SeekDB Bridge] SeekDB initialized successfully
[SeekDB Bridge] Executing: CREATE TABLE IF NOT EXISTS projects (...)
[SeekDB Bridge] Executing: CREATE TABLE IF NOT EXISTS vector_documents (...)
[SeekDB Bridge] Executing: CREATE VECTOR INDEX IF NOT EXISTS idx_embedding ON vector_documents(embedding) ...
[SeekDB Bridge] Executing: CREATE INDEX IF NOT EXISTS idx_project_id ON vector_documents(project_id)...
[SeekDB Bridge] Executing: CREATE INDEX IF NOT EXISTS idx_document_id ON vector_documents(document_id)...
[SeekDB Bridge] Executing: CREATE TABLE IF NOT EXISTS conversations (...)
[SeekDB Bridge] Executing: CREATE TABLE IF NOT EXISTS messages (...)
[SeekDB Bridge] Committing transaction
[SeekDB Bridge] Query returned 0 rows
✅ 应用程序成功启动并运行
```

## 技术要点

### SeekDB/OceanBase Lite 的数据库模型

1. **数据库实例（Instance）**
   - `oblite.open(path)` 打开一个实例
   - 实例是一个目录结构（`oblite.db/`）
   - 包含日志、配置、存储等

2. **数据库（Database/Schema）**
   - 实例中可以有多个数据库
   - 每个数据库独立的命名空间
   - 必须通过 `CREATE DATABASE` 创建

3. **连接行为**
   - `oblite.connect("")` - 系统上下文，可以创建数据库
   - `oblite.connect(db_name)` - 连接到特定数据库
   - 连接成功不代表数据库存在！

### 常见陷阱

1. ❌ **错误**: 认为 `oblite.connect()` 会自动创建数据库
   ```python
   conn = oblite.connect("new_db")  # 不会创建数据库
   ```

2. ❌ **错误**: 依赖异常来检测数据库是否存在
   ```python
   try:
       conn = oblite.connect(db_name)  # 即使数据库不存在也成功
   except:
       create_database()  # 永远不会执行
   ```

3. ✅ **正确**: 主动确保数据库存在
   ```python
   admin_conn = oblite.connect("")
   admin_cursor.execute("CREATE DATABASE IF NOT EXISTS db_name")
   admin_conn.close()
   conn = oblite.connect(db_name)  # 现在安全
   ```

## 相关问题

### 为什么删除旧的 oblite.db 目录后问题仍然存在？

因为即使是全新的实例，也不会自动创建数据库。必须显式执行 `CREATE DATABASE`。

### USE DATABASE 语句有用吗？

测试发现 `USE database` 语句在 SeekDB 中：
- 如果数据库不存在，返回错误 1049 (Unknown database)
- 如果数据库存在，也会返回错误 1210 (Invalid argument)

因此不推荐使用 USE 语句，而是直接通过 `oblite.connect(db_name)` 指定数据库。

### 为什么测试脚本能正常工作？

查看 `test_seekdb.py`，它使用的是临时目录和简单的数据库名，可能：
1. 测试中使用的数据库名恰好与系统默认数据库匹配
2. 或者测试环境有不同的配置

实际生产环境中必须显式创建数据库。

## 总结

- ✅ SeekDB 需要显式创建数据库，不会自动创建
- ✅ 使用 `oblite.connect("")` 获取系统上下文
- ✅ 执行 `CREATE DATABASE IF NOT EXISTS` 确保数据库存在
- ✅ 然后再连接到目标数据库进行操作

## 相关文件

- `src-tauri/python/seekdb_bridge.py` - SeekDB Python 桥接（已修复）
- `src-tauri/src/services/seekdb_adapter.rs` - Rust 适配器
- `docs/FIX_PIP_INSTALL_ERROR.md` - pip 安装问题修复（前置问题）

## 修复日期

2025-10-29

## 连锁问题修复

本次修复解决了以下一系列问题：

1. ✅ pip install 失败 (No such file or directory) - 使用 `python -m pip`
2. ✅ oblite 模块导入失败 - 先导入 seekdb 再导入 oblite
3. ✅ SeekDB 数据库不存在 - 显式创建数据库

应用程序现在可以完全正常启动和运行！🎉

