import React, { useState, useEffect } from 'react';
import { FolderOpen, Clock, Plus, Sun, Moon, Pencil, Trash2, MoreVertical } from 'lucide-react';
import CreateProjectModal from '../ProjectPanel/CreateProjectModal';
import AddDocumentModal from '../ProjectPanel/AddDocumentModal';
import ChatPanel from '../ChatPanel';
import { handleCreateProject } from '../ProjectPanel/projectHandlers';
import { handleAddDocuments } from '../ProjectPanel/documentHandlers';
import { getProjects, ProjectResponse, deleteProject, renameProject } from '../../services/projectService';
import { Theme } from '../../hooks/useTheme';
import ConfirmDialog from '../common/ConfirmDialog';
import RenameDialog from '../common/RenameDialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card } from '@/components/ui/card';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

interface LayoutProps {
  selectedProjectId: string | null;
  onProjectSelect: (projectId: string | null) => void;
  theme: Theme;
  onToggleTheme: () => void;
}

const Layout: React.FC<LayoutProps> = ({ selectedProjectId, onProjectSelect, theme, onToggleTheme }) => {
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isAddDocModalOpen, setIsAddDocModalOpen] = useState(false);
  const [selectedProjectForAdd, setSelectedProjectForAdd] = useState<ProjectResponse | null>(null);
  const [projects, setProjects] = useState<ProjectResponse[]>([]);
  const [isLoadingProjects, setIsLoadingProjects] = useState(false);

  // 删除相关状态
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [projectToDelete, setProjectToDelete] = useState<ProjectResponse | null>(null);

  // 重命名相关状态
  const [isRenameDialogOpen, setIsRenameDialogOpen] = useState(false);
  const [projectToRename, setProjectToRename] = useState<ProjectResponse | null>(null);

  // 添加拖拽相关状态
  const [leftPanelWidth, setLeftPanelWidth] = useState(260); // 像素
  const [isResizing, setIsResizing] = useState(false);

  // 加载项目列表
  const loadProjects = async () => {
    setIsLoadingProjects(true);
    try {
      const projectList = await getProjects();
      setProjects(projectList);
    } catch (error) {
      console.error('加载项目列表失败:', error);
    } finally {
      setIsLoadingProjects(false);
    }
  };

  // 应用启动时加载项目列表
  useEffect(() => {
    loadProjects();
  }, []);

  // 拖拽处理函数
  const handleMouseDown = () => {
    setIsResizing(true);
    // 拖拽时禁止文本选择
    document?.body?.classList?.add('no-select');
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!isResizing) return;

    const newWidth = e?.clientX;

    // 限制宽度范围：200px - 400px
    if (newWidth >= 200 && newWidth <= 400) {
      setLeftPanelWidth(newWidth);
    }
  };

  const handleMouseUp = () => {
    setIsResizing(false);
    // 恢复文本选择
    document?.body?.classList?.remove('no-select');
  };

  useEffect(() => {
    if (isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);

      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        // 清理时也移除类名，防止意外情况
        document?.body?.classList?.remove('no-select');
      };
    }
  }, [isResizing]);

  const handleProjectCreate = async (projectData: { name: string; files?: File[]; filePaths?: string[] }) => {
    return handleCreateProject(projectData, onProjectSelect, loadProjects);
  };

  // 打开添加文档浮层
  const handleOpenAddDocModal = (project: ProjectResponse, e: React.MouseEvent) => {
    e.stopPropagation(); // 阻止事件冒泡，避免触发项目选中
    setSelectedProjectForAdd(project);
    setIsAddDocModalOpen(true);
  };

  // 处理文档添加
  const handleDocumentAdd = async (data: { files?: File[]; filePaths?: string[] }) => {
    if (!selectedProjectForAdd?.id) {
      throw new Error('未选择项目');
    }
    return handleAddDocuments(selectedProjectForAdd.id, data, loadProjects);
  };

  // 格式化时间
  const formatDate = (dateString: string) => {
    try {
      const date = new Date(dateString);
      const now = new Date();
      const diffMs = now.getTime() - date.getTime();
      const diffMins = Math.floor(diffMs / 60000);
      const diffHours = Math.floor(diffMs / 3600000);
      const diffDays = Math.floor(diffMs / 86400000);

      if (diffMins < 1) return '刚刚';
      if (diffMins < 60) return `${diffMins}分钟前`;
      if (diffHours < 24) return `${diffHours}小时前`;
      if (diffDays < 7) return `${diffDays}天前`;

      return date.toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit'
      });
    } catch {
      return dateString;
    }
  };

  // 打开删除确认对话框
  const handleOpenDeleteDialog = (project: ProjectResponse, e: React.MouseEvent) => {
    e.stopPropagation();
    setProjectToDelete(project);
    setIsDeleteDialogOpen(true);
  };

  // 确认删除项目
  const handleConfirmDelete = async () => {
    if (!projectToDelete?.id) return;

    try {
      await deleteProject(projectToDelete.id);

      // 如果删除的是当前选中的项目，清除选中状态
      if (selectedProjectId === projectToDelete.id) {
        onProjectSelect(null);
      }

      // 刷新项目列表
      await loadProjects();
    } catch (error) {
      console.error('删除知识库失败:', error);
      alert('删除知识库失败，请重试');
    } finally {
      setIsDeleteDialogOpen(false);
      setProjectToDelete(null);
    }
  };

  // 打开重命名对话框
  const handleOpenRenameDialog = (project: ProjectResponse, e: React.MouseEvent) => {
    e.stopPropagation();
    setProjectToRename(project);
    setIsRenameDialogOpen(true);
  };

  // 确认重命名项目
  const handleConfirmRename = async (newName: string) => {
    if (!projectToRename?.id) return;

    try {
      await renameProject(projectToRename.id, newName);

      // 刷新项目列表
      await loadProjects();
    } catch (error) {
      console.error('重命名知识库失败:', error);
      alert('重命名知识库失败，请重试');
    } finally {
      setIsRenameDialogOpen(false);
      setProjectToRename(null);
    }
  };

  return (
    <div className="flex h-screen bg-background border-t border-border">
      {/* Left Panel - Project List */}
      <div
        className="bg-card border-r border-border p-4 flex flex-col"
        style={{ width: `${leftPanelWidth}px` }}
      >
        <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <img
                src="/logo.gif"
                alt="Mine KB Logo"
                className="w-9 h-9"
              />
              <h2 className="text-lg font-semibold text-foreground">知识库</h2>
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={onToggleTheme}
              title={theme === 'light' ? '切换到暗色模式' : '切换到亮色模式'}
            >
              {theme === 'light' ? (
                <Moon size={20} className="text-muted-foreground" />
              ) : (
                <Sun size={20} className="text-muted-foreground" />
              )}
            </Button>
          </div>
        <div className="space-y-2 mb-4">
          <Button
            className="w-full"
            onClick={() => setIsCreateModalOpen(true)}
          >
            <Plus size={16} className="mr-1" />
            创建知识库
          </Button>
        </div>

        {/* Project list */}
        <div className="flex-1 overflow-y-auto">
          {isLoadingProjects ? (
            <div className="flex items-center justify-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-2 border-primary border-t-transparent"></div>
            </div>
          ) : projects?.length === 0 ? (
            <div className="text-center text-muted-foreground py-8">
              <FolderOpen size={48} className="mx-auto mb-2 opacity-50" />
              <p className="text-sm">暂无知识库</p>
              <p className="text-xs mt-1">点击上方按钮创建</p>
            </div>
          ) : (
            <div className="space-y-2">
              {projects?.map((project) => (
                <Card
                  key={project?.id}
                  onClick={() => onProjectSelect(project?.id)}
                  className={`group p-3 cursor-pointer transition-all ${
                    selectedProjectId === project?.id
                      ? 'bg-accent border-2 border-primary'
                      : 'bg-secondary border-2 border-transparent hover:border-border hover:bg-accent'
                  }`}
                >
                  <div className="flex items-start justify-between mb-2">
                    <h3 className="text-sm font-medium text-card-foreground truncate flex-1 min-w-0 pr-2">
                      {project?.name}
                    </h3>
                    <div className="flex-shrink-0">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-auto p-1 text-muted-foreground hover:text-foreground relative"
                            style={{ transform: 'translate(8px, -4px)' }}
                            onClick={(e) => e.stopPropagation()}
                          >
                            <MoreVertical size={16} />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-40">
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation();
                              handleOpenAddDocModal(project, e as any);
                            }}
                            className="cursor-pointer"
                          >
                            <Plus size={14} className="mr-2" />
                            添加文档
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation();
                              handleOpenRenameDialog(project, e as any);
                            }}
                            className="cursor-pointer"
                          >
                            <Pencil size={14} className="mr-2" />
                            重命名
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation();
                              handleOpenDeleteDialog(project, e as any);
                            }}
                            className="cursor-pointer text-destructive focus:text-destructive"
                          >
                            <Trash2 size={14} className="mr-2" />
                            删除
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-xs bg-accent/50 dark:border-border/30">
                        {project?.document_count || 0} 文档
                      </Badge>
                    </div>
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <Clock size={12} />
                      <span>{formatDate(project?.updated_at)}</span>
                    </div>
                  </div>
                </Card>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* 拖拽分割线 */}
      <div
        className={`resizer ${isResizing ? 'resizing' : ''}`}
        onMouseDown={handleMouseDown}
      />

      {/* Right Panel - Chat Interface */}
      <ChatPanel
        projectId={selectedProjectId}
        projectName={projects.find((p) => p?.id === selectedProjectId)?.name}
      />

      {/* Create Knowledge Base Modal */}
      <CreateProjectModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onSubmit={handleProjectCreate}
        onProjectCreated={loadProjects}
      />

      {/* Add Document Modal */}
      <AddDocumentModal
        isOpen={isAddDocModalOpen}
        onClose={() => {
          setIsAddDocModalOpen(false);
          setSelectedProjectForAdd(null);
        }}
        onSubmit={handleDocumentAdd}
        projectId={selectedProjectForAdd?.id || ''}
        projectName={selectedProjectForAdd?.name}
        onDocumentsAdded={loadProjects}
      />

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        isOpen={isDeleteDialogOpen}
        onClose={() => {
          setIsDeleteDialogOpen(false);
          setProjectToDelete(null);
        }}
        onConfirm={handleConfirmDelete}
        title="删除知识库"
        message={`确定要删除知识库"${projectToDelete?.name}"吗？此操作不可恢复。`}
        confirmText="删除"
        cancelText="取消"
        variant="danger"
      />

      {/* Rename Dialog */}
      <RenameDialog
        isOpen={isRenameDialogOpen}
        currentName={projectToRename?.name || ''}
        onClose={() => {
          setIsRenameDialogOpen(false);
          setProjectToRename(null);
        }}
        onConfirm={handleConfirmRename}
        title="重命名知识库"
        description="请输入新的知识库名称"
      />
    </div>
  );
};

export default Layout;
