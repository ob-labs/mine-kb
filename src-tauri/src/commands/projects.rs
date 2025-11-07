use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub document_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectResponse {
    pub project: ProjectResponse,
}

#[command]
pub async fn create_project(
    request: CreateProjectRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<CreateProjectResponse, String> {
    log::info!("创建项目请求: {:?}", request);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证输入
    if request.name.trim().is_empty() {
        return Err("项目名称不能为空".to_string());
    }

    // 允许创建空项目（从目录导入时会先创建项目再逐个添加文档）
    if request.file_paths.is_empty() {
        log::info!("创建空项目，文档将稍后添加");
    }

    // 创建项目
    let project_id = {
        let project_service_arc = state.project_service();
        let mut project_service = project_service_arc.lock().await;
        project_service
            .create_project(request.name.clone(), request.description.clone())
            .map_err(|e| format!("创建项目失败: {}", e))?
    };

    log::info!("项目创建成功，ID: {}", project_id);

    // 处理文档上传
    let mut document_count = 0;
    let document_service = state.document_service();

    for file_path in request.file_paths {
        match process_document(project_id, file_path, document_service.clone()).await {
            Ok(_) => {
                document_count += 1;
                log::info!("文档处理成功，项目 {} 文档数量: {}", project_id, document_count);
            }
            Err(e) => {
                log::warn!("文档处理失败: {}", e);
                // 继续处理其他文档，不中断整个流程
            }
        }
    }

    // 更新项目的文档数量并获取项目信息
    let project = {
        let project_service_arc = state.project_service();
        let mut project_service = project_service_arc.lock().await;
        if let Some(project) = project_service.get_project_mut(project_id) {
            project.document_count = document_count;
            project.updated_at = chrono::Utc::now();
        }
        // 保存更新后的项目到数据库
        if let Some(project) = project_service.get_project(project_id) {
            let _ = project_service.save_project_to_db(project);
        }
        project_service
            .get_project(project_id)
            .ok_or_else(|| "项目创建后未找到".to_string())?
            .clone()
    };

    let response = CreateProjectResponse {
        project: ProjectResponse {
            id: project.id.to_string(),
            name: project.name,
            description: project.description,
            status: project.status.to_string(),
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
            document_count,
        },
    };

    log::info!("项目创建完成: {:?}", response);
    Ok(response)
}

/// 处理单个文档
async fn process_document(
    project_id: uuid::Uuid,
    file_path: String,
    document_service: std::sync::Arc<tokio::sync::Mutex<crate::services::document_service::DocumentService>>,
) -> Result<uuid::Uuid, String> {
    use std::path::Path;
    use sha2::{Sha256, Digest};

    // 检查文件是否存在
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    // 获取文件信息
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("无法读取文件信息: {}", e))?;

    let file_size = metadata.len();

    // 计算文件哈希
    let content = std::fs::read(&file_path)
        .map_err(|e| format!("无法读取文件内容: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let content_hash = format!("{:x}", hasher.finalize());

    // 添加文档到服务
    let mut doc_service = document_service.lock().await;
    let document_id = doc_service
        .add_document(project_id, file_path, file_size, content_hash)
        .await
        .map_err(|e| format!("添加文档失败: {}", e))?;

    Ok(document_id)
}

#[command]
pub async fn get_projects(
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<Vec<ProjectResponse>, String> {
    log::info!("获取项目列表");

    // 获取应用状态
    let state = wrapper.get_state().await?;

    let project_service_arc = state.project_service();
    let project_service = project_service_arc.lock().await;
    let projects = project_service.list_projects();

    let response: Vec<ProjectResponse> = projects
        .into_iter()
        .map(|project| ProjectResponse {
            id: project.id.to_string(),
            name: project.name.clone(),
            description: project.description.clone(),
            status: project.status.to_string(),
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
            document_count: project.document_count,
        })
        .collect();

    log::info!("返回 {} 个项目", response.len());
    Ok(response)
}

#[command]
pub async fn get_project_details(
    project_id: String,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<ProjectResponse, String> {
    log::info!("获取项目详情: {}", project_id);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    let project_uuid = uuid::Uuid::parse_str(&project_id)
        .map_err(|_| "无效的项目ID格式".to_string())?;

    let project_service_arc = state.project_service();
    let project_service = project_service_arc.lock().await;
    let project = project_service
        .get_project(project_uuid)
        .ok_or_else(|| "项目未找到".to_string())?;

    let response = ProjectResponse {
        id: project.id.to_string(),
        name: project.name.clone(),
        description: project.description.clone(),
        status: project.status.to_string(),
        created_at: project.created_at.to_rfc3339(),
        updated_at: project.updated_at.to_rfc3339(),
        document_count: project.document_count,
    };

    log::info!("返回项目详情: {}", project.name);
    Ok(response)
}

#[command]
pub async fn delete_project(
    project_id: String,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<bool, String> {
    log::info!("删除项目: {}", project_id);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    let project_uuid = uuid::Uuid::parse_str(&project_id)
        .map_err(|_| "无效的项目ID格式".to_string())?;

    let project_service_arc = state.project_service();
    let mut project_service = project_service_arc.lock().await;
    project_service
        .delete_project(project_uuid)
        .map_err(|e| format!("删除项目失败: {}", e))?;

    log::info!("项目删除成功: {}", project_id);
    Ok(true)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameProjectRequest {
    pub project_id: String,
    pub new_name: String,
}

#[command]
pub async fn rename_project(
    request: RenameProjectRequest,
    wrapper: tauri::State<'_, crate::app_state_wrapper::AppStateWrapper>,
) -> Result<ProjectResponse, String> {
    log::info!("重命名项目: {} -> {}", request.project_id, request.new_name);

    // 获取应用状态
    let state = wrapper.get_state().await?;

    // 验证输入
    if request.new_name.trim().is_empty() {
        return Err("项目名称不能为空".to_string());
    }

    let project_uuid = uuid::Uuid::parse_str(&request.project_id)
        .map_err(|_| "无效的项目ID格式".to_string())?;

    let project_service_arc = state.project_service();
    let mut project_service = project_service_arc.lock().await;

    // 更新项目名称
    project_service
        .update_project(project_uuid, Some(request.new_name.trim().to_string()), None)
        .map_err(|e| format!("重命名项目失败: {}", e))?;

    // 获取更新后的项目信息
    let project = project_service
        .get_project(project_uuid)
        .ok_or_else(|| "项目未找到".to_string())?;

    let response = ProjectResponse {
        id: project.id.to_string(),
        name: project.name.clone(),
        description: project.description.clone(),
        status: project.status.to_string(),
        created_at: project.created_at.to_rfc3339(),
        updated_at: project.updated_at.to_rfc3339(),
        document_count: project.document_count,
    };

    log::info!("项目重命名成功: {}", project.name);
    Ok(response)
}
