import { invoke } from '@tauri-apps/api/tauri';

export interface CreateProjectRequest {
  name: string;
  file_paths: string[];
}

export interface ProjectResponse {
  id: string;
  name: string;
  description?: string;
  status: string;
  created_at: string;
  updated_at: string;
  document_count: number;
}

export interface CreateProjectResponse {
  project: ProjectResponse;
}

/**
 * 将文件保存到临时目录并返回文件路径
 */
export async function getFilePathsForUpload(files: File[]): Promise<string[]> {
  // 导入文件服务中的保存函数
  const { saveFilesToTemp } = await import('./fileService');
  return await saveFilesToTemp(files);
}

/**
 * 创建新项目
 */
export async function createProject(request: CreateProjectRequest): Promise<CreateProjectResponse> {
  try {
    const response = await invoke<CreateProjectResponse>('create_project', { request });
    return response;
  } catch (error) {
    console.error('创建知识库失败:', error);
    throw new Error(`创建知识库失败: ${error}`);
  }
}

/**
 * 获取所有项目列表
 */
export async function getProjects(): Promise<ProjectResponse[]> {
  try {
    const projects = await invoke<ProjectResponse[]>('get_projects');
    return projects;
  } catch (error) {
    console.error('获取项目列表失败:', error);
    throw new Error(`获取项目列表失败: ${error}`);
  }
}

/**
 * 获取项目详情
 */
export async function getProjectDetails(projectId: string): Promise<ProjectResponse> {
  try {
    const project = await invoke<ProjectResponse>('get_project_details', { projectId });
    return project;
  } catch (error) {
    console.error('获取项目详情失败:', error);
    throw new Error(`获取项目详情失败: ${error}`);
  }
}

/**
 * 删除项目
 */
export async function deleteProject(projectId: string): Promise<boolean> {
  try {
    const result = await invoke<boolean>('delete_project', { projectId });
    return result;
  } catch (error) {
    console.error('删除项目失败:', error);
    throw new Error(`删除项目失败: ${error}`);
  }
}

export interface RenameProjectRequest {
  project_id: string;
  new_name: string;
}

/**
 * 重命名项目
 */
export async function renameProject(projectId: string, newName: string): Promise<ProjectResponse> {
  try {
    const request: RenameProjectRequest = {
      project_id: projectId,
      new_name: newName,
    };
    const result = await invoke<ProjectResponse>('rename_project', { request });
    return result;
  } catch (error) {
    console.error('重命名项目失败:', error);
    throw new Error(`重命名项目失败: ${error}`);
  }
}
