use serde::{Deserialize, Serialize};
use tauri::command;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadDocumentsRequest {
    pub project_id: String,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub filename: String,
    pub file_size: u64,
    pub processing_status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadDocumentsResponse {
    pub successful: Vec<DocumentResponse>,
    pub failed: Vec<FailedDocumentInfo>,
    pub summary: UploadSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FailedDocumentInfo {
    pub filename: String,
    pub file_path: String,
    pub error: String,
    pub error_stage: String, // "validation" | "reading" | "processing" | "embedding" | "indexing"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadSummary {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateFilesRequest {
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileValidationInfo {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub mime_type: String,
    pub is_valid: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileValidationError {
    pub path: String,
    pub filename: String,
    pub error: String,
    pub error_type: String, // "not_found" | "too_large" | "empty" | "unsupported_format" | "permission_denied" | "other"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateFilesResponse {
    pub valid: Vec<FileValidationInfo>,
    pub invalid: Vec<FileValidationError>,
    pub summary: ValidationSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total: usize,
    pub valid_count: usize,
    pub invalid_count: usize,
    pub total_size: u64,
}

#[command]
pub async fn upload_documents(
    request: UploadDocumentsRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<UploadDocumentsResponse, String> {
    log::info!("📤 上传文档请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证输入
    if request.file_paths.is_empty() {
        return Err("至少需要上传一个文档".to_string());
    }

    // 解析项目 ID
    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|e| format!("无效的项目ID: {}", e))?;

    // 检查项目是否存在
    {
        let project_service = state.project_service();
        let project_service_guard = project_service.lock().await;
        if project_service_guard.get_project(project_id).is_none() {
            return Err(format!("项目不存在: {}", project_id));
        }
    }

    // 处理文档上传
    let document_service = state.document_service();
    let mut successful_docs = Vec::new();
    let mut failed_docs = Vec::new();
    let total_files = request.file_paths.len();

    for file_path in request.file_paths {
        log::info!("📄 处理文件: {}", file_path);

        match process_single_document(project_id, file_path.clone(), document_service.clone()).await {
            Ok((doc_id, filename, file_size, status, created_at)) => {
                successful_docs.push(DocumentResponse {
                    id: doc_id.to_string(),
                    filename: filename.clone(),
                    file_size,
                    processing_status: status,
                    created_at: created_at.to_rfc3339(),
                });
                log::info!("✅ 文档上传成功: {} (ID: {})", filename, doc_id);
            }
            Err(e) => {
                // 提取文件名
                let filename = std::path::Path::new(&file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("未知文件")
                    .to_string();

                // 解析错误阶段
                let (error_stage, error_message) = parse_error_stage(&e);

                failed_docs.push(FailedDocumentInfo {
                    filename: filename.clone(),
                    file_path: file_path.clone(),
                    error: error_message,
                    error_stage,
                });
                log::error!("❌ 文档上传失败: {} - {}", filename, e);
            }
        }
    }

    // 更新项目的文档数量
    {
        // 先计算文档数量（从数据库查询，确保是累加的总数）
        let doc_count = {
            let doc_service = state.document_service();
            let doc_service_guard = doc_service.lock().await;
            doc_service_guard.count_documents(Some(project_id)).await
        };

        log::info!("📊 项目 {} 的文档总数: {}", project_id, doc_count);

        // 然后更新项目
        let project_service = state.project_service();
        let mut project_service_guard = project_service.lock().await;
        if let Some(project) = project_service_guard.get_project_mut(project_id) {
            project.document_count = doc_count as u32;
            project.updated_at = chrono::Utc::now();

            // 保存更新到数据库
            let project_clone = project.clone();
            let _ = project_service_guard.save_project_to_db(&project_clone);
        }
    }

    let summary = UploadSummary {
        total: total_files,
        successful: successful_docs.len(),
        failed: failed_docs.len(),
    };

    log::info!(
        "🎯 文档上传完成 - 总数: {}, 成功: {}, 失败: {}",
        summary.total,
        summary.successful,
        summary.failed
    );

    // 即使部分失败也返回成功，让前端处理失败列表
    Ok(UploadDocumentsResponse {
        successful: successful_docs,
        failed: failed_docs,
        summary,
    })
}

/// 解析错误信息，提取错误阶段和清晰的错误消息
fn parse_error_stage(error: &str) -> (String, String) {
    if error.contains("[阶段1-验证]") || error.contains("文件不存在") {
        ("validation".to_string(), extract_error_message(error))
    } else if error.contains("[阶段2-元数据]") || error.contains("无法读取文件信息") {
        ("reading".to_string(), extract_error_message(error))
    } else if error.contains("[阶段3-读取]") || error.contains("无法读取文件内容") {
        ("reading".to_string(), extract_error_message(error))
    } else if error.contains("[阶段4-处理]") || error.contains("文档处理失败") {
        ("processing".to_string(), extract_error_message(error))
    } else if error.contains("embedding") || error.contains("向量") {
        ("embedding".to_string(), extract_error_message(error))
    } else if error.contains("[阶段5-查询]") || error.contains("索引") {
        ("indexing".to_string(), extract_error_message(error))
    } else {
        ("unknown".to_string(), error.to_string())
    }
}

/// 提取错误消息的核心部分，去除阶段标记
fn extract_error_message(error: &str) -> String {
    // 移除阶段标记，只保留实际错误信息
    if let Some(pos) = error.find("] ") {
        error[pos + 2..].to_string()
    } else {
        error.to_string()
    }
}

/// 处理单个文档的上传和处理
async fn process_single_document(
    project_id: Uuid,
    file_path: String,
    document_service: Arc<Mutex<crate::services::document_service::DocumentService>>,
) -> Result<(Uuid, String, u64, String, chrono::DateTime<chrono::Utc>), String> {
    use std::path::Path;
    use sha2::{Sha256, Digest};

    log::info!("📄 [阶段1/5] 开始处理文档: {}", file_path);

    // 阶段1: 验证文件存在性
    let path = Path::new(&file_path);
    if !path.exists() {
        let error = format!("[阶段1-验证] 文件不存在: {}", file_path);
        log::error!("❌ {}", error);
        return Err(error);
    }

    // 阶段2: 读取文件元数据
    log::debug!("📋 [阶段2/5] 读取文件元数据...");
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| {
            let error = format!("[阶段2-元数据] 无法读取文件信息: {} - {}", file_path, e);
            log::error!("❌ {}", error);
            error
        })?;

    let file_size = metadata.len();

    // 获取文件名
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            let error = format!("[阶段2-元数据] 无效的文件名: {}", file_path);
            log::error!("❌ {}", error);
            error
        })?
        .to_string();

    log::info!("✅ 文件信息 - 名称: {}, 大小: {} bytes", filename, file_size);

    // 阶段3: 读取文件内容并计算哈希
    log::debug!("🔐 [阶段3/5] 读取文件内容并计算哈希...");
    let content = std::fs::read(&file_path)
        .map_err(|e| {
            let error = format!("[阶段3-读取] 无法读取文件内容: {} - {}", filename, e);
            log::error!("❌ {}", error);
            error
        })?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = format!("{:x}", hasher.finalize());

    log::debug!("✅ 文件哈希: {}", hash);

    // 阶段4: 添加文档到服务（包含文本提取、分块、向量化）
    log::info!("📝 [阶段4/5] 处理文档内容（提取文本、分块、向量化）...");
    let mut doc_service = document_service.lock().await;
    let document_id = doc_service
        .add_document(project_id, file_path.clone(), file_size, hash)
        .await
        .map_err(|e| {
            let error_msg = e.to_string();

            // 根据错误类型提供更详细的错误信息
            let detailed_error = if error_msg.contains("Failed to extract") {
                format!("[阶段4-文本提取] 无法提取文档内容: {} - 可能是文件损坏或格式不正确", filename)
            } else if error_msg.contains("No valid chunks") {
                format!("[阶段4-分块] 文档内容为空或无法分块: {} - 文档可能没有可提取的文本内容", filename)
            } else if error_msg.contains("embedding") || error_msg.contains("API") {
                format!("[阶段4-向量化] 向量化失败: {} - API调用错误或网络问题", filename)
            } else if error_msg.contains("Unsupported file type") {
                format!("[阶段4-格式] 不支持的文件格式: {} - {}", filename, error_msg)
            } else {
                format!("[阶段4-处理] 文档处理失败: {} - {}", filename, error_msg)
            };

            log::error!("❌ {}", detailed_error);
            detailed_error
        })?;

    log::info!("✅ 文档处理成功，ID: {}", document_id);

    // 阶段5: 获取文档信息
    log::debug!("📊 [阶段5/5] 获取文档状态...");
    let document = doc_service
        .get_document(document_id)
        .ok_or_else(|| {
            let error = format!("[阶段5-查询] 文档添加后未找到: {}", filename);
            log::error!("❌ {}", error);
            error
        })?;

    log::info!(
        "🎉 文档处理完成: {} (状态: {}, chunks: {})",
        filename,
        document.processing_status,
        document.chunk_count
    );

    Ok((
        document.id,
        document.filename.clone(),
        document.file_size,
        document.processing_status.to_string(),
        document.created_at,
    ))
}

/// 批量验证文件
/// 在实际处理前进行预检查，快速识别无效文件
#[command]
pub async fn validate_files(
    request: ValidateFilesRequest,
) -> Result<ValidateFilesResponse, String> {
    log::info!("批量验证文件请求: {} 个文件", request.file_paths.len());

    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    let mut total_size: u64 = 0;

    for file_path in request.file_paths {
        match validate_single_file(&file_path).await {
            Ok(info) => {
                total_size += info.size;
                valid.push(info);
            }
            Err(error_info) => {
                invalid.push(error_info);
            }
        }
    }

    let summary = ValidationSummary {
        total: valid.len() + invalid.len(),
        valid_count: valid.len(),
        invalid_count: invalid.len(),
        total_size,
    };

    log::info!(
        "文件验证完成 - 总数: {}, 有效: {}, 无效: {}, 总大小: {} MB",
        summary.total,
        summary.valid_count,
        summary.invalid_count,
        total_size / (1024 * 1024)
    );

    Ok(ValidateFilesResponse {
        valid,
        invalid,
        summary,
    })
}

/// 验证单个文件
async fn validate_single_file(
    file_path: &str,
) -> Result<FileValidationInfo, FileValidationError> {
    use std::path::Path;

    let path = Path::new(file_path);

    // 获取文件名
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    // 检查文件是否存在
    if !path.exists() {
        return Err(FileValidationError {
            path: file_path.to_string(),
            filename,
            error: "文件不存在".to_string(),
            error_type: "not_found".to_string(),
        });
    }

    // 检查是否为文件
    if !path.is_file() {
        return Err(FileValidationError {
            path: file_path.to_string(),
            filename,
            error: "路径不是文件".to_string(),
            error_type: "not_found".to_string(),
        });
    }

    // 获取文件元数据
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            let error_type = if e.kind() == std::io::ErrorKind::PermissionDenied {
                "permission_denied"
            } else {
                "other"
            };
            return Err(FileValidationError {
                path: file_path.to_string(),
                filename,
                error: format!("无法读取文件信息: {}", e),
                error_type: error_type.to_string(),
            });
        }
    };

    let file_size = metadata.len();

    // 检查文件大小
    const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB
    if file_size > MAX_FILE_SIZE {
        return Err(FileValidationError {
            path: file_path.to_string(),
            filename,
            error: format!(
                "文件过大: {:.2} MB (最大: 50 MB)",
                file_size as f64 / (1024.0 * 1024.0)
            ),
            error_type: "too_large".to_string(),
        });
    }

    // 检查文件是否为空
    if file_size == 0 {
        return Err(FileValidationError {
            path: file_path.to_string(),
            filename,
            error: "文件为空".to_string(),
            error_type: "empty".to_string(),
        });
    }

    // 检查文件格式是否支持
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let supported_extensions = vec!["txt", "md", "markdown", "pdf", "doc", "docx", "rtf"];
    if !supported_extensions.contains(&extension.to_lowercase().as_str()) {
        return Err(FileValidationError {
            path: file_path.to_string(),
            filename,
            error: format!(
                "不支持的文件格式: .{} (支持: {})",
                extension,
                supported_extensions.join(", ")
            ),
            error_type: "unsupported_format".to_string(),
        });
    }

    // 检测 MIME 类型
    let mime_type = match extension.to_lowercase().as_str() {
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "rtf" => "application/rtf",
        _ => "application/octet-stream",
    };

    Ok(FileValidationInfo {
        path: file_path.to_string(),
        filename,
        size: file_size,
        mime_type: mime_type.to_string(),
        is_valid: true,
    })
}

#[command]
pub async fn get_document_content(_document_id: String) -> Result<String, String> {
    // TODO: Implement get document content
    Err("Not implemented".to_string())
}
