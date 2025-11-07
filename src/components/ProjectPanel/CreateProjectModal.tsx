import React, { useState, useRef } from 'react';
import { Upload, File, Trash2, FileUp, FolderOpen, AlertCircle } from 'lucide-react';
import { open } from '@tauri-apps/api/shell';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { selectDirectory, scanDirectory, type FileInfo } from '@/services/fileService';

interface CreateProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (projectData: { name: string; files?: File[]; filePaths?: string[] }) => Promise<string>; // 返回项目ID
  onProjectCreated?: () => void; // 项目创建/更新完成后的回调
}

interface ProcessingProgress {
  current: number;
  total: number;
  currentFileName: string;
}

interface FailedFile {
  name: string;
  error: string;
}

const CreateProjectModal: React.FC<CreateProjectModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  onProjectCreated,
}) => {
  const [projectName, setProjectName] = useState('Untitled');
  const [activeTab, setActiveTab] = useState<'upload' | 'directory'>('upload');

  // 上传文件tab的状态
  const [selectedFiles, setSelectedFiles] = useState<File[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // 从目录导入tab的状态
  const [selectedDirectory, setSelectedDirectory] = useState<string>('');
  const [scannedFiles, setScannedFiles] = useState<FileInfo[]>([]);
  const [isScanning, setIsScanning] = useState(false);

  // 处理进度状态
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [processingProgress, setProcessingProgress] = useState<ProcessingProgress | null>(null);
  const [failedFiles, setFailedFiles] = useState<FailedFile[]>([]);
  const [isProcessingComplete, setIsProcessingComplete] = useState(false);

  // 用于中断处理的标志和项目ID
  const shouldAbortRef = useRef(false);
  const createdProjectIdRef = useRef<string | null>(null);

  // 重置表单状态
  const resetForm = () => {
    setProjectName('Untitled');
    setActiveTab('upload');
    setSelectedFiles([]);
    setScannedFiles([]);
    setSelectedDirectory('');
    setIsDragOver(false);
    setIsSubmitting(false);
    setProcessingProgress(null);
    setFailedFiles([]);
    setIsProcessingComplete(false);
    shouldAbortRef.current = false;
    createdProjectIdRef.current = null;

    // 重置文件输入
    if (fileInputRef?.current) {
      fileInputRef.current.value = '';
    }
  };

  // 处理关闭事件
  const handleClose = () => {
    if (isSubmitting && !isProcessingComplete) {
      // 如果正在处理，设置中断标志
      if (window.confirm('正在处理文件，关闭将取消操作并清理已创建的数据。确定要关闭吗？')) {
        shouldAbortRef.current = true;
        // 不立即关闭，等待清理完成
      }
    } else {
      resetForm();
      onClose();
    }
  };

  // 处理有失败文件时的取消操作
  const handleCancelWithCleanup = async () => {
    // 如果是从目录导入且有部分失败，删除整个项目
    if (activeTab === 'directory' && isProcessingComplete && failedFiles.length > 0 && createdProjectIdRef.current) {
      if (window.confirm('取消将删除刚创建的知识库及已添加的文档。确定要取消吗？')) {
        await cleanupProject(createdProjectIdRef.current);
        resetForm();
        onClose();
      }
    } else {
      handleClose();
    }
  };

  // 重试失败的文件
  const handleRetryFailedFiles = async () => {
    if (failedFiles.length === 0 || !createdProjectIdRef.current) return;

    setIsSubmitting(true);
    // 不要设置 isProcessingComplete 为 false，保持失败列表可见

    const totalFiles = failedFiles.length;
    const stillFailed: FailedFile[] = [];
    let retriedSuccessCount = 0;

    try {
      // 逐个重试失败的文件
      for (let i = 0; i < failedFiles.length; i++) {
        const failedFile = failedFiles[i];
        const fileName = failedFile?.name;

        // 从扫描的文件中找到对应的文件路径
        const fileInfo = scannedFiles.find(f => f?.name === fileName);
        if (!fileInfo) {
          stillFailed.push({
            name: fileName,
            error: '无法找到文件路径',
          });
          continue;
        }

        // 更新进度
        setProcessingProgress({
          current: i + 1,
          total: totalFiles,
          currentFileName: fileName,
        });

        // 重试处理文件
        try {
          await processFile(createdProjectIdRef.current, fileInfo.path);
          retriedSuccessCount++;
        } catch (error) {
          console.error(`重试处理文件失败 ${fileName}:`, error);
          stillFailed.push({
            name: fileName,
            error: String(error),
          });
        }
      }

      // 重试完成
      setFailedFiles(stillFailed);
      setProcessingProgress(null);

      // 刷新项目列表
      onProjectCreated?.();

      // 如果全部成功
      if (stillFailed.length === 0) {
        alert(`重试成功！所有文件已处理完成。`);
        setTimeout(() => {
          resetForm();
          onClose();
        }, 1000);
      } else {
        // 仍有失败
        alert(`重试完成。成功: ${retriedSuccessCount}，失败: ${stillFailed.length}`);
      }
    } catch (error) {
      console.error('重试失败:', error);
      alert(`重试失败: ${error}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  // 上传文件相关函数
  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files || []);
    addFiles(files);
  };

  const addFiles = (newFiles: File[]) => {
    const allowedTypes = ['.txt', '.md', '.pdf', '.doc', '.docx', '.rtf'];
    const validFiles = newFiles.filter(file => {
      const extension = '.' + file.name.split('.').pop()?.toLowerCase();
      return allowedTypes.includes(extension);
    });

    // 避免重复文件
    const uniqueFiles = validFiles.filter(newFile =>
      !selectedFiles.some(existingFile =>
        existingFile.name === newFile.name && existingFile.size === newFile.size
      )
    );

    if (uniqueFiles.length > 0) {
      setSelectedFiles(prev => [...prev, ...uniqueFiles]);
    }

    if (validFiles.length < newFiles.length) {
      alert('部分文件格式不支持，仅支持 .txt, .md, .pdf, .doc, .docx, .rtf 格式');
    }
  };

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!e.currentTarget.contains(e.relatedTarget as Node)) {
      setIsDragOver(false);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);

    const files = Array.from(e.dataTransfer.files);
    if (files.length > 0) {
      addFiles(files);
    }
  };

  const handleRemoveFile = (index: number) => {
    setSelectedFiles(prev => prev.filter((_, i) => i !== index));
  };

  // 目录导入相关函数
  const handleSelectDirectory = async () => {
    try {
      setIsScanning(true);
      const dirPath = await selectDirectory();
      setSelectedDirectory(dirPath);

      // 扫描目录
      const files = await scanDirectory(dirPath);
      setScannedFiles(files);

      // 如果文件数量很多，显示警告
      if (files.length > 100) {
        alert(`扫描到 ${files.length} 个文件，处理可能需要较长时间`);
      }
    } catch (error) {
      console.error('选择或扫描目录失败:', error);
      const errorMsg = String(error);
      if (!errorMsg.includes('未选择目录')) {
        alert(`操作失败: ${error}`);
      }
      setSelectedDirectory('');
      setScannedFiles([]);
    } finally {
      setIsScanning(false);
    }
  };

  const handleRemoveScannedFile = (index: number) => {
    setScannedFiles(prev => prev.filter((_, i) => i !== index));
  };

  // 提交处理
  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();

    if (!projectName.trim()) {
      alert('请输入知识库名称');
      return;
    }

    const filesCount = activeTab === 'upload' ? selectedFiles.length : scannedFiles.length;
    if (filesCount === 0) {
      alert('请至少选择一个文件');
      return;
    }

    setIsSubmitting(true);
    setFailedFiles([]);
    setIsProcessingComplete(false);
    shouldAbortRef.current = false;

    try {
      if (activeTab === 'upload') {
        // 上传文件模式：直接调用原有逻辑
        await onSubmit({
          name: projectName.trim(),
          files: selectedFiles,
        });

        // 成功后重置并关闭
        resetForm();
        onClose();
      } else {
        // 从目录导入模式：逐个处理文件并显示进度
        await handleDirectoryImport();
      }
    } catch (error) {
      console.error('创建知识库失败:', error);
      alert(`创建知识库失败: ${error}`);
      setIsSubmitting(false);
    }
  };

  // 处理从目录导入
  const handleDirectoryImport = async () => {
    const filePaths = scannedFiles.map(f => f.path);
    const totalFiles = filePaths.length;
    const failed: FailedFile[] = [];

    try {
      // 首先创建项目（不传文件）
      const projectId = await onSubmit({
        name: projectName.trim(),
        filePaths: [], // 先创建空项目
      });

      createdProjectIdRef.current = projectId;

      // 然后逐个处理文件
      for (let i = 0; i < filePaths.length; i++) {
        // 检查是否需要中断
        if (shouldAbortRef.current) {
          console.log('用户中断处理，开始清理...');
          await cleanupProject(projectId);
          resetForm();
          onClose();
          return;
        }

        const filePath = filePaths[i];
        const fileName = scannedFiles[i]?.name || filePath.split('/').pop() || filePath;

        // 更新进度
        setProcessingProgress({
          current: i + 1,
          total: totalFiles,
          currentFileName: fileName,
        });

        // 处理单个文件
        try {
          await processFile(projectId, filePath);
        } catch (error) {
          console.error(`处理文件失败 ${fileName}:`, error);
          failed.push({
            name: fileName,
            error: String(error),
          });
          // 继续处理下一个文件
        }
      }

      // 所有文件处理完成
      setFailedFiles(failed);
      setIsProcessingComplete(true);

      // 如果全部失败，删除项目
      if (failed.length === totalFiles) {
        alert('所有文件处理失败，已删除项目');
        await cleanupProject(projectId);
        resetForm();
        onClose();
        return;
      }

      // 刷新项目列表以显示最新的文档数量
      onProjectCreated?.();

      // 如果有部分成功，显示结果
      if (failed.length > 0) {
        // 不自动关闭，让用户查看失败列表
      } else {
        // 全部成功，自动关闭
        setTimeout(() => {
          resetForm();
          onClose();
        }, 1000);
      }
    } catch (error) {
      console.error('目录导入失败:', error);
      throw error;
    } finally {
      setIsSubmitting(false);
    }
  };

  // 处理单个文件（调用后端API）
  const processFile = async (projectId: string, filePath: string): Promise<void> => {
    const { invoke } = await import('@tauri-apps/api/tauri');

    const result = await invoke('upload_documents', {
      request: {
        project_id: projectId,
        file_paths: [filePath],
      },
    }) as {
      successful: Array<any>;
      failed: Array<{ filename: string; file_path: string; error: string; error_stage: string }>;
      summary: { total: number; successful: number; failed: number };
    };

    // 如果这个文件处理失败，抛出详细错误
    if (result?.failed && result.failed.length > 0) {
      const failedFile = result.failed[0];
      const errorStageText = getErrorStageText(failedFile?.error_stage);
      throw new Error(`${errorStageText}: ${failedFile?.error}`);
    }

    // 如果没有成功的文件，也抛出错误
    if (!result?.successful || result.successful.length === 0) {
      throw new Error('文件处理失败，未返回结果');
    }
  };

  // 将错误阶段转换为中文描述
  const getErrorStageText = (stage: string): string => {
    const stageMap: Record<string, string> = {
      'validation': '文件验证失败',
      'reading': '文件读取失败',
      'processing': '文档处理失败',
      'embedding': '向量化失败',
      'indexing': '索引失败',
      'unknown': '未知错误',
    };
    return stageMap[stage] || '处理失败';
  };

  // 清理项目（中断时调用）
  const cleanupProject = async (projectId: string): Promise<void> => {
    try {
      const { invoke } = await import('@tauri-apps/api/tauri');
      await invoke('delete_project', { projectId });
      console.log('已清理项目:', projectId);
    } catch (error) {
      console.error('清理项目失败:', error);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const handleFileClick = async (file: File) => {
    try {
      const url = URL.createObjectURL(file);
      await open(url);
    } catch (error) {
      console.error('打开文件失败:', error);
      alert('无法打开文件');
    }
  };

  // 渲染上传文件Tab内容
  const renderUploadTab = () => (
    <TabsContent value="upload" className="space-y-4">
      {/* 文件上传拖拽区域 */}
      <div className="mb-4">
        <input
          ref={fileInputRef}
          type="file"
          multiple
          onChange={handleFileSelect}
          className="hidden"
          accept=".txt,.md,.pdf,.doc,.docx,.rtf"
          disabled={isSubmitting}
        />
        <div
          className={`relative border-2 border-dashed rounded-lg p-8 text-center transition-all duration-200 cursor-pointer ${
            isDragOver
              ? 'border-primary bg-primary/5 text-primary'
              : 'border-border text-foreground hover:border-primary hover:bg-accent'
          } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
          onClick={() => !isSubmitting && fileInputRef?.current?.click()}
          onDragEnter={handleDragEnter}
          onDragLeave={handleDragLeave}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
        >
          <div className="flex flex-col items-center gap-3">
            {isDragOver ? (
              <FileUp size={48} className="text-primary" />
            ) : (
              <Upload size={48} className="text-muted-foreground" />
            )}
            <div>
              <p className="text-lg font-medium mb-1">
                {isDragOver ? '释放文件以上传' : '拖拽文件到此处或点击选择'}
              </p>
              <p className="text-sm text-muted-foreground">
                支持 .txt, .md, .pdf, .doc, .docx, .rtf 格式
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* 已选择的文件列表 */}
      {selectedFiles.length > 0 && (
        <div
          className={`border rounded-md max-h-60 overflow-y-auto bg-secondary transition-all duration-200 ${
            isDragOver ? 'border-primary' : 'border-border'
          }`}
          onDragEnter={handleDragEnter}
          onDragLeave={handleDragLeave}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
        >
          <div className="sticky top-0 p-3 bg-muted border-b border-border z-10">
            <span className="text-sm font-medium text-foreground">
              已选 {selectedFiles.length} 个文件 {isDragOver && '(可继续拖入更多文件)'}
            </span>
          </div>
          <div className="divide-y divide-border">
            {selectedFiles.map((file, index) => (
              <div key={index} className="flex items-center justify-between p-3 hover:bg-accent">
                <div className="flex items-center gap-3 flex-1 min-w-0">
                  <File size={16} className="text-muted-foreground flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <button
                      type="button"
                      onClick={() => handleFileClick(file)}
                      className="text-sm font-medium text-primary hover:opacity-80 truncate block w-full text-left transition-opacity"
                      title="点击打开文件"
                    >
                      {file.name}
                    </button>
                    <p className="text-xs text-muted-foreground">
                      {formatFileSize(file.size)}
                    </p>
                  </div>
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => handleRemoveFile(index)}
                  className="text-destructive hover:text-destructive/80 h-auto p-1"
                  disabled={isSubmitting}
                >
                  <Trash2 size={16} />
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}
    </TabsContent>
  );

  // 渲染从目录导入Tab内容
  const renderDirectoryTab = () => (
    <TabsContent value="directory" className="space-y-4">
      {/* 选择目录按钮 */}
      <div className="mb-4">
        <Button
          type="button"
          onClick={handleSelectDirectory}
          disabled={isSubmitting || isScanning}
          className="w-full h-24"
          variant="outline"
        >
          <div className="flex flex-col items-center gap-2">
            <FolderOpen size={32} />
            <span>{isScanning ? '扫描中...' : '选择文档目录'}</span>
            {selectedDirectory && (
              <span className="text-xs text-muted-foreground truncate max-w-full">
                {selectedDirectory}
              </span>
            )}
          </div>
        </Button>
      </div>

      {/* 扫描到的文件列表 */}
      {scannedFiles.length > 0 && (
        <div className="border rounded-md max-h-60 overflow-y-auto bg-secondary">
          <div className="sticky top-0 p-3 bg-muted border-b border-border z-10">
            {processingProgress ? (
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="font-medium text-foreground">
                    正在处理 {processingProgress.currentFileName}
                  </span>
                  <span className="text-muted-foreground">
                    {processingProgress.current}/{processingProgress.total}
                  </span>
                </div>
                <div className="w-full bg-background rounded-full h-1.5">
                  <div
                    className="bg-primary h-1.5 rounded-full transition-all duration-300"
                    style={{
                      width: `${(processingProgress.current / processingProgress.total) * 100}%`,
                    }}
                  />
                </div>
              </div>
            ) : (
              <span className="text-sm font-medium text-foreground">
                已扫描 {scannedFiles.length} 个文件
              </span>
            )}
          </div>
          <div className="divide-y divide-border">
            {scannedFiles.map((file, index) => (
              <div key={index} className="flex items-center justify-between p-3 hover:bg-accent">
                <div className="flex items-center gap-3 flex-1 min-w-0">
                  <File size={16} className="text-muted-foreground flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate" title={file.path}>
                      {file.name}
                    </p>
                    <p className="text-xs text-muted-foreground truncate" title={file.path}>
                      {file.path} · {formatFileSize(file.size)}
                    </p>
                  </div>
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => handleRemoveScannedFile(index)}
                  className="text-destructive hover:text-destructive/80 h-auto p-1"
                  disabled={isSubmitting}
                >
                  <Trash2 size={16} />
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 失败文件列表 */}
      {isProcessingComplete && failedFiles.length > 0 && (
        <div className="border border-destructive rounded-md p-4 bg-destructive/5">
          <div className="flex items-start justify-between gap-2 mb-2">
            <div className="flex items-start gap-2 flex-1">
              <AlertCircle size={18} className="text-destructive flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <span className="text-sm font-medium text-destructive block mb-1">
                  以下 {failedFiles.length} 个文件处理失败：
                </span>
                {/* 重试进度 */}
                {isSubmitting && processingProgress && (
                  <div className="space-y-1 mt-2">
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">
                        正在重试: {processingProgress?.currentFileName}
                      </span>
                      <span className="text-muted-foreground">
                        {processingProgress?.current}/{processingProgress?.total}
                      </span>
                    </div>
                    <div className="w-full bg-background rounded-full h-1">
                      <div
                        className="bg-destructive h-1 rounded-full transition-all duration-300"
                        style={{
                          width: `${((processingProgress?.current || 0) / (processingProgress?.total || 1)) * 100}%`,
                        }}
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={handleRetryFailedFiles}
              disabled={isSubmitting}
              className="border-destructive text-destructive hover:bg-destructive hover:text-destructive-foreground flex-shrink-0"
            >
              {isSubmitting ? '重试中...' : '重试'}
            </Button>
          </div>
          <div className="space-y-1 max-h-40 overflow-y-auto">
            {failedFiles.map((file, index) => (
              <div key={index} className="text-xs text-muted-foreground pl-6">
                <span className="font-medium">{file?.name}</span> - {file?.error}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 处理完成提示 */}
      {isProcessingComplete && failedFiles.length === 0 && (
        <div className="border border-green-500 rounded-md p-4 bg-green-500/5">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-green-700">
              ✓ 所有文件处理成功！
            </span>
          </div>
        </div>
      )}
    </TabsContent>
  );

  return (
    <Dialog open={isOpen} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-2xl max-h-[90vh] flex flex-col p-0">
        <div className="p-6 pb-0">
          <DialogHeader>
            <DialogTitle>创建知识库</DialogTitle>
          </DialogHeader>
        </div>
        <form id="create-project-form" onSubmit={handleSubmit} className="flex-1 overflow-hidden">
          <div className="space-y-6 overflow-y-auto max-h-[60vh] px-6">
            {/* 项目名称 */}
            <div className="px-1">
              <label htmlFor="projectName" className="block text-sm font-medium text-foreground mb-2">
                知识库名称 <span className="text-destructive">*</span>
              </label>
              <Input
                type="text"
                id="projectName"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                placeholder="输入知识库名称"
                disabled={isSubmitting}
                required
                className="focus-visible:ring-2"
              />
            </div>

            {/* Tab切换 */}
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                添加文档 <span className="text-destructive">*</span>
              </label>
              <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'upload' | 'directory')}>
                <TabsList className="grid w-full grid-cols-2">
                  <TabsTrigger value="upload" disabled={isSubmitting}>
                    上传文件
                  </TabsTrigger>
                  <TabsTrigger value="directory" disabled={isSubmitting}>
                    从目录导入
                  </TabsTrigger>
                </TabsList>

                {renderUploadTab()}
                {renderDirectoryTab()}
              </Tabs>
            </div>
          </div>
        </form>
        <div className="p-6 pt-0">
          <DialogFooter>
            <Button
              type={isProcessingComplete && failedFiles.length > 0 ? 'button' : 'submit'}
              form={isProcessingComplete && failedFiles.length > 0 ? undefined : 'create-project-form'}
              onClick={isProcessingComplete && failedFiles.length > 0 ? handleClose : undefined}
              disabled={
                isSubmitting ||
                !projectName.trim() ||
                (activeTab === 'upload' && selectedFiles.length === 0) ||
                (activeTab === 'directory' && scannedFiles.length === 0)
              }
            >
              {isSubmitting ? (
                <>
                  <div className="animate-spin rounded-full h-4 w-4 border-2 border-current border-t-transparent mr-2"></div>
                  {activeTab === 'directory' ? '处理中...' : '创建中...'}
                </>
              ) : isProcessingComplete ? (
                '完成'
              ) : (
                '创建'
              )}
            </Button>
            <Button
              variant="outline"
              onClick={handleCancelWithCleanup}
              disabled={isSubmitting && !isProcessingComplete}
            >
              {isProcessingComplete ? '关闭' : '取消'}
            </Button>
          </DialogFooter>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default CreateProjectModal;