# SeekDB 升级指南：0.0.1.dev2 → 0.0.1.dev4

## 📋 概述

本文档记录了将 mine-kb 项目中的 SeekDB 从 0.0.1.dev2 升级到 0.0.1.dev4 的详细过程和变更说明。

**升级日期**: 2025-11-05  
**升级版本**: seekdb 0.0.1.dev2 → 0.0.1.dev4

---

## 🔄 主要变更

### 1. 模块重命名

**最重要的变化**：`oblite` 模块已更名为 `seekdb`

**变更前（0.0.1.dev2）**:
```python
import seekdb  # seekdb 包
import oblite  # 实际使用的模块

oblite.open(db_path)
conn = oblite.connect(db_name)
```

**变更后（0.0.1.dev4）**:
```python
import seekdb  # seekdb 包，直接使用

seekdb.open(db_path)
conn = seekdb.connect(db_name)
```

### 2. 新增特性

#### 2.1 向量列类型输出支持
0.0.1.dev4 版本支持直接输出向量（vector）列类型，无需额外转换。

#### 2.2 数据库存在性验证
`connect()` 方法现在会验证数据库是否存在：
- 如果数据库不存在，会抛出错误
- 如果未指定数据库名，默认连接到 "test" 数据库
- 支持 `connect.close()` 方法

**示例**:
```python
import seekdb

seekdb.open("./mydb.db")

# 方式1: 连接到已存在的数据库
conn = seekdb.connect("mine_kb")  # 如果 mine_kb 不存在，会报错

# 方式2: 先创建数据库，再连接
admin_conn = seekdb.connect("")  # 连接到管理上下文
cursor = admin_conn.cursor()
cursor.execute("CREATE DATABASE IF NOT EXISTS mine_kb")
admin_conn.commit()
admin_conn.close()

conn = seekdb.connect("mine_kb")  # 现在可以安全连接
```

#### 2.3 USE 语句支持
现在支持标准的 `USE database` 语法：

```python
cursor.execute("USE mine_kb")  # 切换到指定数据库
```

#### 2.4 自动提交模式
支持在连接时指定自动提交模式：

```python
# 手动提交（默认）
conn = seekdb.connect(db_name='mine_kb')
cursor.execute("INSERT INTO ...")
conn.commit()  # 需要手动提交

# 自动提交模式
conn = seekdb.connect(db_name='mine_kb', autocommit=True)
cursor.execute("INSERT INTO ...")  # 自动提交
```

---

## 📦 安装方式

### 使用清华镜像源安装
```bash
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple
```

### 在虚拟环境中安装（推荐）

#### Linux/macOS
```bash
# 创建虚拟环境
python3 -m venv ~/.local/share/com.mine-kb.app/venv

# 激活虚拟环境
source ~/.local/share/com.mine-kb.app/venv/bin/activate

# 安装 seekdb
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple

# 验证安装
python -c "import seekdb; print('SeekDB 安装成功！')"
```

#### Windows
```powershell
# 创建虚拟环境
python -m venv %APPDATA%\com.mine-kb.app\venv

# 激活虚拟环境
%APPDATA%\com.mine-kb.app\venv\Scripts\activate

# 安装 seekdb
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple

# 验证安装
python -c "import seekdb; print('SeekDB 安装成功！')"
```

### 自动安装脚本

项目提供了自动安装脚本：
```bash
cd src-tauri/python
bash install_deps.sh
```

该脚本会自动：
1. 检测 Python 3 安装
2. 创建虚拟环境（如果不存在）
3. 安装 seekdb==0.0.1.dev4
4. 验证安装是否成功

---

## 📝 API 变化对照表

| 功能 | 0.0.1.dev2 | 0.0.1.dev4 |
|------|-----------|-----------|
| **导入模块** | `import oblite` | `import seekdb` |
| **打开数据库** | `oblite.open(path)` | `seekdb.open(path)` |
| **连接数据库** | `oblite.connect(db_name)` | `seekdb.connect(db_name)` |
| **自动提交** | 不支持 | `seekdb.connect(db_name='test', autocommit=True)` |
| **USE 语句** | 不稳定 | `cursor.execute("USE database")` 稳定支持 |
| **向量输出** | 需要转换 | 原生支持 vector 列类型输出 |
| **数据库验证** | 不验证 | 自动验证数据库是否存在 |
| **关闭连接** | `conn.close()` | `conn.close()` + `connect.close()` |

---

## 🔧 升级步骤

### 步骤 1: 更新依赖版本
更新 `src-tauri/python/requirements.txt`:
```txt
seekdb==0.0.1.dev4
```

### 步骤 2: 更新代码中的导入语句
**查找并替换**所有代码中的：
- `import oblite` → `import seekdb`
- `oblite.open()` → `seekdb.open()`
- `oblite.connect()` → `seekdb.connect()`

### 步骤 3: 更新数据库路径（可选）
建议将数据库文件名从 `oblite.db` 更新为 `mine_kb.db`：
```python
# 旧路径
db_path = "~/.local/share/mine-kb/oblite.db"

# 新路径（推荐）
db_path = "~/.local/share/mine-kb/mine_kb.db"
```

### 步骤 4: 重新安装依赖
```bash
# 在虚拟环境中
pip uninstall seekdb -y
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple
```

### 步骤 5: 测试验证
运行测试脚本验证升级：
```bash
cd src-tauri/python
python test_seekdb.py
```

---

## 📂 已更新的文件列表

### 依赖配置文件
- ✅ `src-tauri/python/requirements.txt` - 版本号更新为 0.0.1.dev4
- ✅ `src-tauri/python/install_deps.sh` - 安装脚本更新

### 核心代码
- ✅ `src-tauri/python/seekdb_bridge.py` - 导入语句和 API 调用更新

### 测试脚本
- ✅ `src-tauri/python/test_seekdb.py` - 测试脚本更新
- ✅ `scripts/test_oblite_upsert.py` → 重命名为 `test_seekdb_upsert.py`（建议）

### 工具脚本
- ✅ `src-tauri/python/migrate_sqlite_to_seekdb.py` - 迁移脚本更新
- ✅ `scripts/debug_db_data.py` - 调试脚本更新
- ✅ `scripts/verify_message_order.py` - 使用 seekdb_bridge，无需修改

---

## ⚠️ 注意事项

### 1. 数据库兼容性
- 现有的数据库文件（.db）**完全兼容**，无需迁移数据
- 数据表结构保持不变
- 向量索引保持不变

### 2. 向后兼容性
- 旧代码中的 `import oblite` 将**无法工作**
- 必须更新所有导入语句为 `import seekdb`

### 3. 数据库创建
0.0.1.dev4 版本对数据库存在性要求更严格：
```python
# ❌ 错误：如果数据库不存在会报错
conn = seekdb.connect("nonexistent_db")

# ✅ 正确：先创建数据库
admin_conn = seekdb.connect("")
admin_conn.cursor().execute("CREATE DATABASE IF NOT EXISTS my_db")
admin_conn.commit()
admin_conn.close()
conn = seekdb.connect("my_db")
```

### 4. 虚拟环境
**强烈建议**使用虚拟环境：
- 避免污染系统 Python 环境
- 便于管理依赖版本
- 提高应用隔离性

### 5. 测试建议
升级后务必测试以下功能：
- ✅ 数据库连接和初始化
- ✅ 基本 CRUD 操作
- ✅ 向量搜索功能
- ✅ 事务提交和回滚
- ✅ 多线程/多进程访问

---

## 🐛 常见问题

### Q1: 升级后出现 "ModuleNotFoundError: No module named 'oblite'"
**原因**: 代码中仍有 `import oblite` 语句未更新  
**解决**: 使用全局搜索，将所有 `import oblite` 替换为 `import seekdb`

### Q2: 数据库连接报错 "Database does not exist"
**原因**: 0.0.1.dev4 会验证数据库是否存在  
**解决**: 在连接前先创建数据库（参见注意事项 3）

### Q3: 虚拟环境中找不到 seekdb
**原因**: seekdb 未安装在正确的虚拟环境中  
**解决**: 
```bash
# 确认虚拟环境已激活
which python  # Linux/macOS
where python  # Windows

# 重新安装
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple
```

### Q4: 向量搜索性能下降
**原因**: 向量索引可能需要重建  
**解决**:
```sql
DROP INDEX idx_embedding;
CREATE VECTOR INDEX idx_embedding ON vector_documents(embedding) 
WITH (distance=l2, type=hnsw, lib=vsag);
```

---

## 📚 相关资源

- **SeekDB 官方文档**: (待补充)
- **清华镜像源**: https://pypi.tuna.tsinghua.edu.cn/simple
- **项目 GitHub**: (待补充)

---

## 📞 技术支持

如果在升级过程中遇到问题，请：
1. 查看本文档的"常见问题"章节
2. 运行 `python test_seekdb.py` 诊断问题
3. 检查虚拟环境是否正确激活
4. 提交 Issue 到项目仓库

---

## ✅ 升级检查清单

- [ ] 更新 `requirements.txt` 中的版本号
- [ ] 更新所有代码中的 `import oblite` 为 `import seekdb`
- [ ] 更新所有 `oblite.open()` 为 `seekdb.open()`
- [ ] 更新所有 `oblite.connect()` 为 `seekdb.connect()`
- [ ] 在虚拟环境中安装 seekdb==0.0.1.dev4
- [ ] 运行 `test_seekdb.py` 验证安装
- [ ] 测试数据库连接功能
- [ ] 测试向量搜索功能
- [ ] 测试现有数据读写
- [ ] 更新部署文档

---

**文档版本**: 1.0  
**最后更新**: 2025-11-05  
**维护者**: mine-kb 开发团队

