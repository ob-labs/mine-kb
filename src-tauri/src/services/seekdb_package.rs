use anyhow::{anyhow, Result};
use std::process::Command;
use super::python_env::PythonEnv;

const SEEKDB_VERSION: &str = "0.0.1.dev4";
const PYPI_INDEX: &str = "https://pypi.tuna.tsinghua.edu.cn/simple/";

/// SeekDB 包管理器
pub struct SeekDbPackage<'a> {
    python_env: &'a PythonEnv,
}

impl<'a> SeekDbPackage<'a> {
    /// 创建新的 SeekDB 包管理器
    pub fn new(python_env: &'a PythonEnv) -> Self {
        Self { python_env }
    }
    
    /// 检查 seekdb 包是否已安装
    pub fn is_installed(&self) -> Result<bool> {
        log::info!("🔍 检查 seekdb 包是否已安装...");
        
        let output = Command::new(self.python_env.get_python_executable())
            .arg("-c")
            .arg("import seekdb; print(seekdb.__file__)")
            .output();
        
        match output {
            Ok(output) => {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout);
                    log::info!("✅ seekdb 已安装: {}", path.trim());
                    Ok(true)
                } else {
                    log::info!("⚠️  seekdb 未安装");
                    Ok(false)
                }
            }
            Err(e) => {
                log::warn!("检查 seekdb 安装状态失败: {}", e);
                Ok(false)
            }
        }
    }
    
    /// 安装 seekdb 包
    pub fn install(&self) -> Result<()> {
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("  📦 安装 SeekDB 包");
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("   版本: {}", SEEKDB_VERSION);
        log::info!("   镜像: {}", PYPI_INDEX);
        log::info!("");
        log::info!("这可能需要几分钟时间，请稍候...");
        
        let python_executable = self.python_env.get_python_executable();
        
        // 首先升级 pip
        log::info!("🔧 升级 pip...");
        let upgrade_pip = Command::new(python_executable)
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--upgrade")
            .arg("pip")
            .arg("-i")
            .arg(PYPI_INDEX)
            .status();
        
        match upgrade_pip {
            Ok(status) if status.success() => {
                log::info!("✅ pip 升级完成");
            }
            _ => {
                log::warn!("⚠️  pip 升级失败，继续安装 seekdb...");
            }
        }
        
        // 安装 seekdb
        log::info!("📦 安装 seekdb=={}...", SEEKDB_VERSION);
        
        let status = Command::new(python_executable)
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg(format!("seekdb=={}", SEEKDB_VERSION))
            .arg("-i")
            .arg(PYPI_INDEX)
            .status()
            .map_err(|e| anyhow!("执行 pip install 失败: {}", e))?;
        
        if !status.success() {
            return Err(anyhow!(
                "seekdb 安装失败（退出码: {:?}）\n\n\
                请检查：\n\
                1. 网络连接是否正常\n\
                2. 镜像源是否可访问: {}\n\
                3. 系统架构是否支持 seekdb\n\n\
                您也可以手动安装：\n\
                {:?} -m pip install seekdb=={} -i {}",
                status.code(),
                PYPI_INDEX,
                python_executable,
                SEEKDB_VERSION,
                PYPI_INDEX
            ));
        }
        
        log::info!("✅ seekdb 安装完成");
        Ok(())
    }
    
    /// 验证 seekdb 安装
    pub fn verify(&self) -> Result<()> {
        log::info!("🔍 验证 seekdb 安装...");
        
        // 尝试导入 seekdb 模块（0.0.1.dev4 版本已移除 oblite 模块）
        let output = Command::new(self.python_env.get_python_executable())
            .arg("-c")
            .arg("import seekdb; print('seekdb location:', seekdb.__file__)")
            .output()
            .map_err(|e| anyhow!("验证 seekdb 失败: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "seekdb 验证失败\n\n\
                无法导入 seekdb 模块\n\
                错误信息: {}\n\n\
                请尝试重新安装：\n\
                {:?} -m pip install --force-reinstall seekdb=={} -i {}",
                stderr.trim(),
                self.python_env.get_python_executable(),
                SEEKDB_VERSION,
                PYPI_INDEX
            ));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::info!("✅ seekdb 验证通过");
        for line in stdout.lines() {
            log::info!("   {}", line);
        }
        
        Ok(())
    }
    
    /// 获取 seekdb 版本信息
    pub fn get_version_info(&self) -> Result<String> {
        let output = Command::new(self.python_env.get_python_executable())
            .arg("-c")
            .arg(format!(
                "try:\n    import seekdb\n    print('{}')\nexcept:\n    print('unknown')",
                SEEKDB_VERSION
            ))
            .output()
            .map_err(|e| anyhow!("获取版本信息失败: {}", e))?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("unknown".to_string())
        }
    }
}

