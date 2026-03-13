use serde::{Deserialize, Serialize};
use tauri::command;
use tauri::State;
use tauri::api::dialog::blocking::FileDialogBuilder;
use std::path::Path;
use std::fs;
use std::io::Write;
use base64::Engine;
use crate::AppDataDirPath;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppStatusResponse {
    pub status: String,
    pub version: String,
    pub database_status: String,
    pub vector_db_status: String,
    pub llm_service_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigureLLMRequest {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
}

/// 返回当前使用的应用数据目录（与后端一致；前端用于 temp 等路径）
#[command]
pub fn get_app_data_dir(app_data_dir: State<'_, AppDataDirPath>) -> Result<String, String> {
    Ok(app_data_dir.0.clone())
}

/// 请求体：前端使用 camelCase（contentBase64），用 serde 对齐
#[derive(Debug, Deserialize)]
pub struct SaveFileToAppTmpArgs {
    pub filename: String,
    #[serde(rename = "contentBase64")]
    pub content_base64: String,
}

/// 将文件内容写入应用数据目录下的 tmp，避免前端 fs scope 限制（开发时 CONFIG_DIR 与 $APPDATA 不一致）。
/// 返回写入后的绝对路径。
#[command]
pub fn save_file_to_app_tmp(
    app_data_dir: State<'_, AppDataDirPath>,
    args: SaveFileToAppTmpArgs,
) -> Result<String, String> {
    if args.filename.contains("..") || args.filename.contains('/') || args.filename.contains('\\') {
        return Err("filename must not contain path segments".to_string());
    }
    let tmp_dir = Path::new(&app_data_dir.0).join("tmp");
    if !tmp_dir.exists() {
        fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    }
    let file_path = tmp_dir.join(&args.filename);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&args.content_base64)
        .map_err(|e| format!("base64 decode: {}", e))?;
    let mut f = fs::File::create(&file_path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    file_path
        .into_os_string()
        .into_string()
        .map_err(|_| "path not UTF-8".to_string())
}

#[command]
pub async fn get_app_status() -> Result<AppStatusResponse, String> {
    // TODO: Implement get app status
    Err("Not implemented".to_string())
}

#[command]
pub async fn configure_llm_service(_request: ConfigureLLMRequest) -> Result<bool, String> {
    // TODO: Implement configure LLM service
    Err("Not implemented".to_string())
}

/// 打开目录选择对话框
#[command]
pub async fn select_directory() -> Result<String, String> {
    log::info!("打开目录选择对话框");

    let result = FileDialogBuilder::new()
        .set_title("选择文档目录")
        .pick_folder();

    match result {
        Some(path) => {
            let path_str = path.to_string_lossy().to_string();
            log::info!("选中目录: {}", path_str);
            Ok(path_str)
        }
        None => Err("未选择目录".to_string()),
    }
}

/// 递归扫描目录，返回所有支持的文档文件
#[command]
pub async fn scan_directory(dir_path: String) -> Result<Vec<FileInfo>, String> {
    log::info!("开始扫描目录: {}", dir_path);

    let path = Path::new(&dir_path);

    if !path.exists() {
        return Err(format!("目录不存在: {}", dir_path));
    }

    if !path.is_dir() {
        return Err(format!("路径不是目录: {}", dir_path));
    }

    let allowed_extensions = vec!["txt", "md", "pdf", "doc", "docx", "rtf"];
    let mut files = Vec::new();

    match scan_directory_recursive(path, &allowed_extensions, &mut files) {
        Ok(_) => {
            log::info!("扫描完成，找到 {} 个文件", files.len());

            if files.is_empty() {
                return Err("未找到支持的文档格式（.txt, .md, .pdf, .doc, .docx, .rtf）".to_string());
            }

            // 如果文件数量很多，记录警告
            if files.len() > 100 {
                log::warn!("扫描到 {} 个文件，处理可能需要较长时间", files.len());
            }

            Ok(files)
        }
        Err(e) => Err(format!("扫描目录失败: {}", e)),
    }
}

/// 递归扫描目录的辅助函数
fn scan_directory_recursive(
    dir: &Path,
    allowed_extensions: &[&str],
    files: &mut Vec<FileInfo>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("无法读取目录 {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // 记录错误但继续处理
                log::warn!("读取目录项失败: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // 如果是目录，递归扫描
        if path.is_dir() {
            // 跳过隐藏目录和特殊目录
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') ||
                   name_str == "node_modules" ||
                   name_str == "target" ||
                   name_str == "dist" {
                    log::debug!("跳过目录: {}", path.display());
                    continue;
                }
            }

            // 递归扫描子目录，如果失败记录警告但继续
            if let Err(e) = scan_directory_recursive(&path, allowed_extensions, files) {
                log::warn!("扫描子目录失败: {}", e);
            }
            continue;
        }

        // 检查文件扩展名
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            if allowed_extensions.contains(&ext.as_str()) {
                // 获取文件大小
                match fs::metadata(&path) {
                    Ok(metadata) => {
                        let file_size = metadata.len();
                        let file_name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();

                        files.push(FileInfo {
                            path: path.to_string_lossy().to_string(),
                            name: file_name,
                            size: file_size,
                        });
                    }
                    Err(e) => {
                        log::warn!("无法读取文件元数据 {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(())
}
