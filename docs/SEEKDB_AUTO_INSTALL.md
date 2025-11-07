# SeekDB 自动安装功能（通过 pip）

> **版本**: SeekDB 0.0.1.dev4  
> **最后更新**: 2025-11-05

> **重要更新**: 从 0.0.1.dev4 版本开始，模块名称从 `oblite` 更改为 `seekdb`。详见 [升级指南](UPGRADE_SEEKDB_0.0.1.dev4.md)

## 概述

本应用使用 Python 虚拟环境自动管理 SeekDB 依赖。首次启动时，应用会：
1. 自动创建独立的 Python 虚拟环境
2. 通过 pip 自动安装 seekdb 包（0.0.1.dev4 版本）
3. 验证安装成功后启动应用

无需手动下载或管理依赖文件，一切都是自动完成的。

## 实施架构

### 核心模块

#### 1. `src-tauri/src/services/python_env.rs`
Python 虚拟环境管理器，负责：
- 在应用数据目录创建 Python 虚拟环境
- 检测虚拟环境是否存在
- 提供虚拟环境 Python 可执行文件路径
- 提供 pip 可执行文件路径

**关键方法：**
- `new(app_data_dir)` - 创建环境管理器实例
- `ensure_venv()` - 确保虚拟环境存在，不存在则创建
- `venv_exists()` - 检查虚拟环境是否存在
- `get_python_executable()` - 获取虚拟环境的 Python 路径
- `get_pip_executable()` - 获取虚拟环境的 pip 路径

#### 2. `src-tauri/src/services/seekdb_package.rs`
SeekDB 包管理器，负责：
- 检测 seekdb 包是否已安装
- 自动安装 seekdb 包
- 验证安装是否成功

**关键方法：**
- `new(python_env)` - 创建包管理器实例
- `is_installed()` - 检查 seekdb 是否已安装
- `install()` - 安装 seekdb 包
- `verify()` - 验证安装成功
- `get_version_info()` - 获取版本信息

### 修改的模块

#### `src-tauri/src/services/python_subprocess.rs`
- 修改为 `new_with_python(script_path, python_executable)`
- 直接使用虚拟环境的 Python，不再需要设置 PYTHONPATH
- 移除了所有 PYTHONPATH 相关逻辑

#### `src-tauri/src/services/seekdb_adapter.rs`
- 修改为 `new_with_python(db_path, python_executable)`
- 接收 Python 可执行文件路径参数
- 传递给 PythonSubprocess

#### `src-tauri/src/services/document_service.rs`
- 修改为 `with_full_config(db_path, api_key, base_url, python_path)`
- 传递 Python 可执行文件路径

#### `src-tauri/src/services/app_state.rs`
- 修改为 `new_with_full_config(db_path, app_config, model_cache_dir, python_path)`
- 传递 Python 可执行文件路径给所有服务

#### `src-tauri/src/main.rs`
应用启动流程（三个阶段）：

**阶段 1：Python 环境和 SeekDB 安装**
1. 创建 Python 虚拟环境管理器
2. 确保虚拟环境存在（不存在则创建）
3. 检查 seekdb 是否已安装
4. 未安装则自动安装
5. 验证安装成功
6. 获取 Python 可执行文件路径

**阶段 2：配置文件加载**
- 加载应用配置
- 验证 API 密钥等

**阶段 3：初始化应用状态**
- 传递 Python 路径给各个服务
- 初始化数据库连接

### 移除的模块

- ❌ `src-tauri/src/services/seekdb_installer.rs` - 不再需要
- ❌ `src-tauri/libs/` 目录 - 不再需要手动管理 oblite.so

## 技术要点

### 安装配置

- **包名**：`seekdb`
- **版本**：`0.0.1.dev4`
- **镜像源**：`https://pypi.tuna.tsinghua.edu.cn/simple/`
- **安装位置**：`<应用数据目录>/venv/`
- **安装方式**：`pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/`

### 虚拟环境位置

不同操作系统的虚拟环境位置：
- **macOS**: `~/Library/Application Support/com.mine-kb.app/venv/`
- **Linux**: `~/.local/share/com.mine-kb.app/venv/`
- **Windows**: `%APPDATA%\com.mine-kb.app\venv\`

### Python 可执行文件路径

- **macOS/Linux**: `<venv_dir>/bin/python3`
- **Windows**: `<venv_dir>\Scripts\python.exe`

### 验证流程

1. 检查虚拟环境是否存在
2. 尝试导入 seekdb 模块（0.0.1.dev4 版本使用 `import seekdb`）
3. 获取 seekdb 模块路径和版本
4. 启动 Python 子进程验证数据库连接

### 优势

1. **跨平台兼容**：pip 自动安装适合当前架构的包（ARM64/x86-64）
2. **依赖隔离**：虚拟环境不影响系统 Python
3. **自动化**：首次运行自动安装，无需用户干预
4. **节省空间**：不需要在项目中存储 2.7GB 的 oblite.so
5. **易于升级**：pip 可以轻松升级到新版本

## 启动日志示例

成功启动时的日志输出：

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  步骤 1/3: 初始化 Python 环境和 SeekDB
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔍 查找 Python 虚拟环境...
   系统 Python: Python 3.10.12
🔧 创建 Python 虚拟环境...
   位置: /home/user/.local/share/com.mine-kb.app/venv
✅ Python 虚拟环境创建成功

🔍 检查 seekdb 包是否已安装...
📦 SeekDB 未安装，开始安装...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  📦 安装 SeekDB 包
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   版本: 0.0.1.dev4
   镜像: https://pypi.tuna.tsinghua.edu.cn/simple/

🔧 升级 pip...
✅ pip 升级完成
📦 安装 seekdb==0.0.1.dev4...
✅ seekdb 安装完成

🔍 验证 seekdb 安装...
✅ seekdb 验证通过
   seekdb version: 0.0.1.dev4
   seekdb path: /path/to/venv/lib/python3.10/site-packages/seekdb/

✅ Python 可执行文件: /path/to/venv/bin/python3

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  步骤 2/3: 加载配置文件
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
成功从配置文件读取配置: /path/to/config.json

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  步骤 3/3: 初始化应用状态
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📦 初始化应用状态...
  - Python 路径: /path/to/venv/bin/python3
🐍 Starting Python subprocess...
✅ Python subprocess started successfully
🔍 验证 SeekDB 数据库连接...
✅ SeekDB 数据库连接正常

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ✅ 应用启动成功！
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## 手动安装（可选）

如果需要手动安装依赖，可以运行：

```bash
cd src-tauri/python
./install_deps.sh
```

此脚本会：
1. 检查 Python 3 是否安装
2. 在应用数据目录创建虚拟环境
3. 激活虚拟环境
4. 升级 pip
5. 安装 seekdb 包

## 测试建议

### 1. 首次安装测试
- 删除应用数据目录（完全清理）
- 启动应用
- 验证自动创建虚拟环境和安装 seekdb

### 2. 重启测试
- 正常关闭应用
- 再次启动应用
- 验证跳过安装，直接使用已有环境

### 3. 网络故障测试
- 删除虚拟环境
- 断开网络连接
- 启动应用
- 验证显示友好的错误信息

### 4. 多架构测试
- 在 ARM64 系统上测试
- 在 x86-64 系统上测试
- 验证 pip 自动安装正确架构的包

## 故障排查

如果应用启动失败，检查以下内容：

### 1. 检查 Python 环境
```bash
python3 --version  # 确保 Python 3.8+
python3 -m venv --help  # 确保 venv 模块可用
```

Ubuntu/Debian 系统可能需要安装：
```bash
sudo apt install python3-venv
```

### 2. 检查网络连接
```bash
ping pypi.tuna.tsinghua.edu.cn
curl -I https://pypi.tuna.tsinghua.edu.cn/simple/
```

### 3. 检查虚拟环境
```bash
# Linux/macOS
ls -la ~/.local/share/com.mine-kb.app/venv/

# 手动测试（0.0.1.dev4 版本使用 seekdb 模块）
~/.local/share/com.mine-kb.app/venv/bin/python3 -c "import seekdb; print(seekdb.__file__)"
```

### 4. 手动安装 seekdb
```bash
# 创建虚拟环境
python3 -m venv ~/.local/share/com.mine-kb.app/venv

# 激活虚拟环境
source ~/.local/share/com.mine-kb.app/venv/bin/activate

# 安装 seekdb 0.0.1.dev4
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/

# 验证（0.0.1.dev4 使用 seekdb 模块）
python -c "import seekdb; print('SeekDB 0.0.1.dev4 OK')"
```

### 5. 查看应用日志
应用日志会显示详细的错误信息，包括：
- Python 版本检测
- 虚拟环境创建过程
- pip 安装过程
- seekdb 验证结果

## 相关文件清单

### 新增文件
- `src-tauri/src/services/python_env.rs` - Python 虚拟环境管理
- `src-tauri/src/services/seekdb_package.rs` - SeekDB 包管理

### 修改文件
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/services/python_subprocess.rs`
- `src-tauri/src/services/seekdb_adapter.rs`
- `src-tauri/src/services/document_service.rs`
- `src-tauri/src/services/app_state.rs`
- `src-tauri/src/main.rs`
- `src-tauri/python/install_deps.sh`

### 删除文件
- `src-tauri/src/services/seekdb_installer.rs` - 已移除
- `src-tauri/libs/` - 已移除

### 归档文件
- `docs/archive/ERROR_ANALYSIS_OBLITE_SO.md` - 旧的错误分析文档（已过时）

---

**更新日期**：2025-11-05  
**版本**：v3.0 (SeekDB 0.0.1.dev4)  
**变更**：
- 升级到 SeekDB 0.0.1.dev4 版本
- 模块名称从 oblite 更改为 seekdb
- 支持向量列输出和数据库验证新特性
