# 修复 pip 安装错误

## 问题描述

应用启动时出现以下错误：
```
执行 pip install 失败: No such file or directory (os error 2)
```

## 根本原因

1. **虚拟环境中缺少 pip 可执行文件**
   - Python 虚拟环境被创建后，`venv/bin/` 目录中只有 python 相关的符号链接
   - 没有生成 `pip` 或 `pip3` 可执行文件
   - 代码尝试直接调用 `pip3` 可执行文件，导致 "No such file or directory" 错误

2. **oblite 模块导入问题**
   - SeekDB 包安装后，`oblite` 模块不能直接导入
   - 必须先导入 `seekdb` 模块来触发 `oblite` 的动态加载
   - `oblite.so` 被缓存到 `~/.seekdb/cache/` 目录

## 修复方案

### 1. 使用 `python -m pip` 替代直接调用 pip

**修改文件**: `src-tauri/src/services/seekdb_package.rs`

将所有 pip 调用改为使用 `python -m pip` 的方式：

```rust
// 修改前
let status = Command::new(&pip_executable)
    .arg("install")
    .arg(format!("seekdb=={}", SEEKDB_VERSION))
    .status()?;

// 修改后
let status = Command::new(python_executable)
    .arg("-m")
    .arg("pip")
    .arg("install")
    .arg(format!("seekdb=={}", SEEKDB_VERSION))
    .status()?;
```

**原因**: `python -m pip` 是更可靠的方式，不依赖于 pip 可执行文件的存在。

### 2. 确保虚拟环境中 pip 可用

**修改文件**: `src-tauri/src/services/python_env.rs`

添加 `ensure_pip()` 方法，在虚拟环境创建后确保 pip 可用：

```rust
fn ensure_pip(&self) -> Result<()> {
    // 检查 pip 是否可用
    let output = Command::new(&self.python_executable)
        .arg("-m")
        .arg("pip")
        .arg("--version")
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            // pip 已可用
            Ok(())
        }
        _ => {
            // 使用 ensurepip 安装 pip
            let install_output = Command::new(&self.python_executable)
                .arg("-m")
                .arg("ensurepip")
                .arg("--default-pip")
                .output()?;
            
            if !install_output.status.success() {
                return Err(anyhow!("pip 安装失败"));
            }
            Ok(())
        }
    }
}
```

### 3. 修复 seekdb/oblite 模块导入顺序

**修改文件**: 
- `src-tauri/src/services/seekdb_package.rs`
- `src-tauri/python/seekdb_bridge.py`

修改验证代码，先导入 `seekdb` 再导入 `oblite`：

```python
# seekdb_bridge.py
try:
    import seekdb  # 先导入 seekdb 来触发 oblite 的加载
    import oblite
except ImportError as e:
    # 错误处理
    ...
```

```rust
// seekdb_package.rs - verify()
let output = Command::new(self.python_env.get_python_executable())
    .arg("-c")
    .arg("import seekdb; import oblite; print('seekdb location:', seekdb.__file__); print('oblite location:', oblite.__file__)")
    .output()
```

## 修复验证

修复后的正常日志输出：

```
✅ Python 虚拟环境已存在
🔍 检查 pip 是否可用...
✅ pip 已可用: pip 25.3 from ...
🔧 升级 pip...
✅ pip 升级完成
📦 安装 seekdb==0.0.1.dev2...
Successfully installed seekdb-0.0.1.dev2 seekdb_lib-0.0.1.dev2
🔍 验证 seekdb 安装...
✅ seekdb 验证通过
[SeekDB Bridge] SeekDB Bridge started, waiting for commands...
[SeekDB Bridge] Initializing SeekDB: path=...
[SeekDB Bridge] SeekDB initialized successfully
```

## 技术要点

1. **`python -m pip` vs 直接调用 pip**
   - `python -m pip` 更可靠，适用于各种环境
   - 不依赖于 pip 可执行文件的存在和路径配置

2. **虚拟环境的 pip 安装**
   - 某些 Python 安装可能不包含完整的 ensurepip
   - 使用 `python -m ensurepip` 可以确保 pip 可用

3. **SeekDB 的模块加载机制**
   - `oblite` 模块是动态加载的
   - 必须先导入 `seekdb` 模块
   - `oblite.so` 会被缓存到用户目录

## 相关文件

- `src-tauri/src/services/seekdb_package.rs` - SeekDB 包管理
- `src-tauri/src/services/python_env.rs` - Python 环境管理
- `src-tauri/python/seekdb_bridge.py` - SeekDB Python 桥接

## 修复日期

2025-10-29

