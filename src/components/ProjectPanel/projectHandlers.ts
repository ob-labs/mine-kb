import { createProject, getFilePathsForUpload } from '../../services/projectService';
import { validateFileType, validateFileSize } from '../../services/fileService';

export const handleCreateProject = async (
  projectData: { name: string; files?: File[]; filePaths?: string[] },
  onProjectSelect: (projectId: string | null) => void,
  onProjectCreated?: () => void
): Promise<string> => {
  try {
    let filePaths: string[] = [];

    // 如果提供了files，验证并保存到临时目录
    if (projectData?.files && projectData.files.length > 0) {
      // 验证文件
      for (const file of projectData.files) {
        if (!validateFileType(file)) {
          throw new Error(`文件 ${file.name} 格式不支持`);
        }
        if (!validateFileSize(file)) {
          throw new Error(`文件 ${file.name} 大小超过限制（最大50MB）`);
        }
      }

      // 将文件保存到临时目录并获取文件路径
      filePaths = await getFilePathsForUpload(projectData.files);
    } else if (projectData?.filePaths) {
      // 如果直接提供了文件路径（从目录导入）
      filePaths = projectData.filePaths;
    }

    // 创建知识库
    const response = await createProject({
      name: projectData.name,
      file_paths: filePaths,
    });

    console.log('项目创建成功:', response);

    const projectId = response?.project?.id;

    // 选择新创建的项目
    onProjectSelect(projectId);

    // 触发列表刷新
    onProjectCreated?.();

    return projectId;
  } catch (error) {
    console.error('创建知识库失败:', error);
    throw error; // 重新抛出错误，让Modal组件处理
  }
};
