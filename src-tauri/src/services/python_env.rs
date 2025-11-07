use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Python 虚拟环境管理器
pub struct PythonEnv {
    venv_dir: PathBuf,
    python_executable: PathBuf,
}

impl PythonEnv {
    /// 创建新的 Python 环境管理器
    pub fn new(app_data_dir: &Path) -> Result<Self> {
        let venv_dir = app_data_dir.join("venv");
        
        // 确定虚拟环境中的 Python 可执行文件路径
        #[cfg(target_os = "windows")]
        let python_executable = venv_dir.join("Scripts").join("python.exe");
        
        #[cfg(not(target_os = "windows"))]
        let python_executable = venv_dir.join("bin").join("python3");
        
        Ok(Self {
            venv_dir,
            python_executable,
        })
    }
    
    /// 检查虚拟环境是否存在
    pub fn venv_exists(&self) -> bool {
        self.venv_dir.exists() && self.python_executable.exists()
    }
    
    /// 确保虚拟环境存在，如果不存在则创建
    pub fn ensure_venv(&self) -> Result<()> {
        if self.venv_exists() {
            log::info!("✅ Python 虚拟环境已存在: {:?}", self.venv_dir);
            return Ok(());
        }
        
        log::info!("🔧 创建 Python 虚拟环境...");
        log::info!("   位置: {:?}", self.venv_dir);
        
        // 检查系统 Python 是否存在
        self.check_system_python()?;
        
        // 创建虚拟环境
        self.create_venv()?;
        
        // 验证虚拟环境
        if !self.venv_exists() {
            return Err(anyhow!(
                "虚拟环境创建失败\n\
                预期位置: {:?}\n\
                Python 可执行文件: {:?}",
                self.venv_dir,
                self.python_executable
            ));
        }
        
        // 确保 pip 可用
        self.ensure_pip()?;
        
        log::info!("✅ Python 虚拟环境创建成功");
        Ok(())
    }
    
    /// 检查系统 Python 是否可用
    fn check_system_python(&self) -> Result<()> {
        let output = Command::new("python3")
            .arg("--version")
            .output();
        
        match output {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    log::info!("   系统 Python: {}", version.trim());
                    Ok(())
                } else {
                    Err(anyhow!("Python3 未正确安装"))
                }
            }
            Err(_) => {
                Err(anyhow!(
                    "未找到 Python3\n\n\
                    请先安装 Python 3.8 或更高版本：\n\
                    - Ubuntu/Debian: sudo apt install python3 python3-venv\n\
                    - macOS: brew install python3\n\
                    - Windows: 从 python.org 下载安装"
                ))
            }
        }
    }
    
    /// 创建虚拟环境
    fn create_venv(&self) -> Result<()> {
        log::info!("   执行: python3 -m venv {:?}", self.venv_dir);
        
        let output = Command::new("python3")
            .arg("-m")
            .arg("venv")
            .arg(&self.venv_dir)
            .output()
            .map_err(|e| anyhow!("创建虚拟环境失败: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // 检查是否是 python3-venv 缺失的问题
            let is_venv_missing = stderr.contains("ensurepip is not available") 
                || stderr.contains("python3-venv");
            
            let error_msg = if is_venv_missing {
                format!(
                    "虚拟环境创建失败：缺少 python3-venv 模块\n\n\
                    请先安装 python3-venv：\n\
                    Ubuntu/Debian: sudo apt install python3-venv\n\
                    或: sudo apt install python3.10-venv\n\n\
                    详细错误信息：\n{}",
                    stderr.trim()
                )
            } else {
                format!(
                    "虚拟环境创建失败（退出码: {:?}）\n\n\
                    标准错误输出：\n{}\n\
                    标准输出：\n{}",
                    output.status.code(),
                    stderr.trim(),
                    stdout.trim()
                )
            };
            
            return Err(anyhow!(error_msg));
        }
        
        Ok(())
    }
    
    /// 确保 pip 可用
    fn ensure_pip(&self) -> Result<()> {
        log::info!("🔍 检查 pip 是否可用...");
        
        // 尝试运行 python -m pip --version
        let output = Command::new(&self.python_executable)
            .arg("-m")
            .arg("pip")
            .arg("--version")
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                log::info!("✅ pip 已可用: {}", version.trim());
                Ok(())
            }
            _ => {
                log::warn!("⚠️  pip 不可用，尝试使用 ensurepip 安装...");
                
                // 使用 ensurepip 模块安装 pip
                let install_output = Command::new(&self.python_executable)
                    .arg("-m")
                    .arg("ensurepip")
                    .arg("--default-pip")
                    .output()
                    .map_err(|e| anyhow!("安装 pip 失败: {}", e))?;
                
                if !install_output.status.success() {
                    let stderr = String::from_utf8_lossy(&install_output.stderr);
                    return Err(anyhow!(
                        "pip 安装失败\n\n\
                        错误信息：\n{}\n\n\
                        请尝试手动安装：\n\
                        1. {:?} -m ensurepip --default-pip\n\
                        或\n\
                        2. curl https://bootstrap.pypa.io/get-pip.py | {:?}",
                        stderr.trim(),
                        self.python_executable,
                        self.python_executable
                    ));
                }
                
                log::info!("✅ pip 安装成功");
                Ok(())
            }
        }
    }
    
    /// 获取虚拟环境的 Python 可执行文件路径
    pub fn get_python_executable(&self) -> &Path {
        &self.python_executable
    }
    
    /// 获取虚拟环境的 pip 可执行文件路径
    pub fn get_pip_executable(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        let pip = self.venv_dir.join("Scripts").join("pip.exe");
        
        #[cfg(not(target_os = "windows"))]
        let pip = self.venv_dir.join("bin").join("pip3");
        
        pip
    }
    
    /// 获取虚拟环境目录
    pub fn get_venv_dir(&self) -> &Path {
        &self.venv_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_python_env_creation() {
        let temp_dir = env::temp_dir().join("test_python_env");
        let python_env = PythonEnv::new(&temp_dir).unwrap();
        
        assert!(python_env.get_venv_dir().to_string_lossy().contains("venv"));
        assert!(python_env.get_python_executable().to_string_lossy().contains("python"));
    }
}

