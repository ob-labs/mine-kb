import { saveFilesToTemp, uploadDocuments } from '../../services/fileService';

/**
 * 处理文档添加逻辑
 * @param projectId 项目ID
 * @param data 文档数据：files（上传文件模式）或 filePaths（从目录导入模式）
 * @param onSuccess 成功回调（用于刷新项目列表）
 */
export async function handleAddDocuments(
  projectId: string,
  data: { files?: File[]; filePaths?: string[] },
  onSuccess?: () => Promise<void>
): Promise<void> {
  try {
    let filePaths: string[] = [];

    // 判断模式
    if (data?.files && data.files.length > 0) {
      // 上传文件模式
      console.log(`开始为项目 ${projectId} 添加 ${data.files.length} 个文档（上传模式）`);

      // 将文件保存到临时目录
      filePaths = await saveFilesToTemp(data.files);
      console.log('文件已保存到临时目录:', filePaths);
    } else if (data?.filePaths && data.filePaths.length > 0) {
      // 从目录导入模式（直接使用文件路径）
      console.log(`开始为项目 ${projectId} 添加 ${data.filePaths.length} 个文档（目录导入模式）`);
      filePaths = data.filePaths;
    } else {
      throw new Error('未提供文件或文件路径');
    }

    // 调用后端上传接口
    const response = await uploadDocuments({
      project_id: projectId,
      file_paths: filePaths,
    });

    console.log('文档上传完成:', response);

    // 执行成功回调（刷新项目列表）
    if (onSuccess) {
      await onSuccess();
    }

    // 检查是否有失败的文件
    if (response?.failed && response.failed.length > 0) {
      // 如果全部失败
      if (response.failed.length === response?.summary?.total) {
        const errorMessages = response.failed.map(f => `${f?.filename}: ${f?.error}`).join('\n');
        throw new Error(`所有文件上传失败:\n${errorMessages}`);
      }

      // 如果部分失败，记录警告并返回部分成功
      const errorMessages = response.failed.map(f => `${f?.filename}: ${f?.error}`).join('\n');
      console.warn(`部分文件上传失败 (${response.failed.length}/${response?.summary?.total}):\n${errorMessages}`);
    }

    // 返回成功
    return Promise.resolve();
  } catch (error) {
    console.error('添加文档失败:', error);
    throw error;
  }
}

