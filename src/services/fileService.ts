import { invoke } from '@tauri-apps/api/tauri';

export interface UploadDocumentsRequest {
  project_id: string;
  file_paths: string[];
}

export interface DocumentResponse {
  id: string;
  filename: string;
  file_size: number;
  processing_status: string;
  created_at: string;
}

export interface FailedDocumentInfo {
  filename: string;
  file_path: string;
  error: string;
  error_stage: string;
}

export interface UploadDocumentsResponse {
  successful: DocumentResponse[];
  failed: FailedDocumentInfo[];
  summary: {
    total: number;
    successful: number;
    failed: number;
  };
}

export interface FileInfo {
  path: string;
  name: string;
  size: number;
}

/**
 * 将文件保存到临时目录并返回路径。
 * 通过后端命令写入 app_data_dir/tmp，避免前端 fs scope 与 CONFIG_DIR 路径不一致导致开发时报错。
 */
export async function saveFilesToTemp(files: File[]): Promise<string[]> {
  const filePaths: string[] = [];

  try {
    for (const file of files) {
      try {
        const timestamp = Date.now();
        const randomId = Math.random().toString(36).substring(2, 15);
        const fileName = `${timestamp}_${randomId}_${file.name}`;

        const arrayBuffer = await file.arrayBuffer();
        const bytes = new Uint8Array(arrayBuffer);
        let binary = '';
        for (let i = 0; i < bytes.length; i++) {
          binary += String.fromCharCode(bytes[i]);
        }
        const contentBase64 = btoa(binary);

        const filePath = await invoke<string>('save_file_to_app_tmp', {
          args: { filename: fileName, contentBase64 },
        });
        filePaths.push(filePath);
        console.log(`文件已保存到: ${filePath}`);
      } catch (error) {
        console.error(`保存文件 ${file.name} 失败:`, error);
        throw new Error(`保存文件 ${file.name} 失败: ${error}`);
      }
    }
    return filePaths;
  } catch (error) {
    console.error('创建临时目录失败:', error);
    throw new Error(`创建临时目录失败: ${error}`);
  }
}

/**
 * 简化版本：直接使用文件名作为路径（用于开发测试）
 */
export async function getFilePathsForUpload(files: File[]): Promise<string[]> {
  // 在实际应用中，这里应该将文件保存到临时目录
  // 现在我们只返回文件名，后端需要相应处理
  return files.map(file => file.name);
}

/**
 * 上传文档到项目
 * 返回详细的上传结果，包括成功和失败的文件
 */
export async function uploadDocuments(request: UploadDocumentsRequest): Promise<UploadDocumentsResponse> {
  try {
    const response = await invoke<UploadDocumentsResponse>('upload_documents', { request });

    // 如果有失败的文件，记录到控制台
    if (response?.failed && response.failed.length > 0) {
      console.warn(`上传完成，但有 ${response.failed.length} 个文件失败:`, response.failed);
    }

    return response;
  } catch (error) {
    console.error('上传文档失败:', error);
    throw new Error(`上传文档失败: ${error}`);
  }
}

/**
 * 获取文档内容
 */
export async function getDocumentContent(documentId: string): Promise<string> {
  try {
    const content = await invoke<string>('get_document_content', { documentId });
    return content;
  } catch (error) {
    console.error('获取文档内容失败:', error);
    throw new Error(`获取文档内容失败: ${error}`);
  }
}

/**
 * 验证文件类型
 */
export function validateFileType(file: File): boolean {
  const allowedTypes = [
    'text/plain',
    'text/markdown',
    'application/pdf',
    'application/msword',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    'application/rtf'
  ];

  const allowedExtensions = ['.txt', '.md', '.pdf', '.doc', '.docx', '.rtf'];

  return allowedTypes.includes(file.type) ||
         allowedExtensions.some(ext => file.name.toLowerCase().endsWith(ext));
}

/**
 * 验证文件大小（最大50MB）
 */
export function validateFileSize(file: File, maxSizeMB: number = 50): boolean {
  const maxSizeBytes = maxSizeMB * 1024 * 1024;
  return file.size <= maxSizeBytes;
}

/**
 * 打开目录选择对话框
 */
export async function selectDirectory(): Promise<string> {
  try {
    const dirPath = await invoke<string>('select_directory');
    return dirPath;
  } catch (error) {
    console.error('选择目录失败:', error);
    throw new Error(`选择目录失败: ${error}`);
  }
}

/**
 * 扫描目录，返回所有支持的文档文件
 */
export async function scanDirectory(dirPath: string): Promise<FileInfo[]> {
  try {
    const files = await invoke<FileInfo[]>('scan_directory', { dirPath });
    return files;
  } catch (error) {
    console.error('扫描目录失败:', error);
    throw new Error(`扫描目录失败: ${error}`);
  }
}
