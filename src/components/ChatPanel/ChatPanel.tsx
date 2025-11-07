import React, { useState, useEffect, useRef } from 'react';
import { MessageCircle, Plus, Trash2, Send, Loader2, PanelLeftClose, PanelLeftOpen, Mic, MicOff, ChevronDown, ChevronRight, FileText, Pencil, MoreVertical, Eraser, Copy, Check } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { open } from '@tauri-apps/api/shell';
import {
  createConversation,
  getConversations,
  getConversationHistory,
  sendMessageStream,
  deleteConversation,
  deleteMessage,
  clearMessages,
  renameConversation,
  type Conversation,
  type Message,
  type MessageSource,
} from '../../services/chatService';
import ConfirmDialog from '../common/ConfirmDialog';
import RenameDialog from '../common/RenameDialog';
import { useVoiceRecorder } from '../../hooks/useVoiceRecorder';
import { recognizeSpeech } from '../../services/speechRecognitionService';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Card } from '@/components/ui/card';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

interface ChatPanelProps {
  projectId: string | null;
  projectName?: string;
}

const ChatPanel: React.FC<ChatPanelProps> = ({ projectId, projectName }) => {
  // 状态管理
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [selectedConversationId, setSelectedConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputMessage, setInputMessage] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [isLoadingConversations, setIsLoadingConversations] = useState(false);

  // 添加拖拽相关状态
  const [conversationListWidth, setConversationListWidth] = useState(260); // 像素
  const [isResizing, setIsResizing] = useState(false);
  const [isConversationListCollapsed, setIsConversationListCollapsed] = useState(false);

  // 确认对话框状态
  const [deleteMessageDialog, setDeleteMessageDialog] = useState<{
    isOpen: boolean;
    messageId: string | null;
    conversationId: string | null;
  }>({
    isOpen: false,
    messageId: null,
    conversationId: null,
  });
  const [isDeletingMessage, setIsDeletingMessage] = useState(false);

  const [deleteConversationDialog, setDeleteConversationDialog] = useState<{
    isOpen: boolean;
    conversationId: string | null;
  }>({
    isOpen: false,
    conversationId: null,
  });
  const [isDeletingConversation, setIsDeletingConversation] = useState(false);

  const [clearMessagesDialog, setClearMessagesDialog] = useState<{
    isOpen: boolean;
    conversationId: string | null;
  }>({
    isOpen: false,
    conversationId: null,
  });
  const [isClearingMessages, setIsClearingMessages] = useState(false);

  const [renameConversationDialog, setRenameConversationDialog] = useState<{
    isOpen: boolean;
    conversationId: string | null;
    currentTitle: string;
  }>({
    isOpen: false,
    conversationId: null,
    currentTitle: '',
  });
  const [isRenamingConversation, setIsRenamingConversation] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // 语音识别状态
  const [isRecognizing, setIsRecognizing] = useState(false);

  // 临时消息的来源信息
  const [tempMessageSources, setTempMessageSources] = useState<MessageSource[]>([]);

  // 来源文档展开/收起状态（持久化）
  const [expandedSources, setExpandedSources] = useState<Set<string>>(() => {
    try {
      const stored = localStorage.getItem('expanded-sources');
      if (stored) {
        return new Set(JSON.parse(stored));
      }
    } catch (error) {
      console.error('加载展开状态失败:', error);
    }
    return new Set();
  });

  // 复制状态：跟踪哪条消息已被复制
  const [copiedMessageId, setCopiedMessageId] = useState<string | null>(null);

  // 使用录音Hook
  const {
    isRecording,
    isSupported: isVoiceSupported,
    toggleRecording,
  } = useVoiceRecorder({
    onRecordingComplete: async (audioBlob: Blob) => {
      console.log('录音完成，开始识别...');
      setIsRecognizing(true);

      try {
        const recognizedText = await recognizeSpeech(audioBlob);

        if (recognizedText) {
          // 将识别的文字追加到输入框
          setInputMessage((prev) => {
            const newText = prev ? `${prev} ${recognizedText}` : recognizedText;
            return newText;
          });

          // 聚焦到输入框
          inputRef?.current?.focus();
        }
      } catch (error) {
        console.error('语音识别失败:', error);
        const errorMsg = error instanceof Error ? error.message : String(error);
        alert(`语音识别失败: ${errorMsg}`);
      } finally {
        setIsRecognizing(false);
      }
    },
    onError: (error: string) => {
      console.error('录音错误:', error);
      alert(error);
    },
  });

  // 自动滚动到底部
  const scrollToBottom = () => {
    messagesEndRef?.current?.scrollIntoView({ behavior: 'smooth' });
  };

  // 加载对话列表
  const loadConversations = async () => {
    if (!projectId) return;

    setIsLoadingConversations(true);
    try {
      const convs = await getConversations(projectId);
      setConversations(convs);

      // 如果有对话，自动选中第一个（按更新时间排序后的第一个）
      if (convs?.length > 0) {
        setSelectedConversationId(convs[0]?.id);
      }
    } catch (error) {
      console.error('加载对话列表失败:', error);
    } finally {
      setIsLoadingConversations(false);
    }
  };

  // 加载对话历史
  const loadMessages = async (conversationId: string) => {
    setIsLoading(true);
    try {
      const msgs = await getConversationHistory(conversationId);
      // sources 已经从后端数据库加载，无需额外处理
      setMessages(msgs);
      setTimeout(scrollToBottom, 100);
    } catch (error) {
      console.error('加载消息失败:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // 创建新对话
  const handleCreateConversation = async () => {
    if (!projectId) return;

    try {
      const newConv = await createConversation(projectId);
      setConversations((prev) => [newConv, ...prev]);
      setSelectedConversationId(newConv.id);
      setMessages([]);
    } catch (error) {
      console.error('创建对话失败:', error);
      alert('创建对话失败');
    }
  };

  // 打开删除对话确认对话框
  const handleDeleteConversationClick = (conversationId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setDeleteConversationDialog({
      isOpen: true,
      conversationId,
    });
  };

  // 确认删除对话
  const confirmDeleteConversation = async () => {
    const { conversationId } = deleteConversationDialog;
    if (!conversationId) return;

    setIsDeletingConversation(true);
    try {
      await deleteConversation(conversationId);
      setConversations((prev) => prev.filter((c) => c?.id !== conversationId));

      if (selectedConversationId === conversationId) {
        setSelectedConversationId(null);
        setMessages([]);
      }

      // 关闭对话框
      setDeleteConversationDialog({ isOpen: false, conversationId: null });
    } catch (error) {
      console.error('删除对话失败:', error);
      alert('删除对话失败');
    } finally {
      setIsDeletingConversation(false);
    }
  };

  // 取消删除对话
  const cancelDeleteConversation = () => {
    setDeleteConversationDialog({ isOpen: false, conversationId: null });
  };

  // 打开重命名对话框
  const handleRenameConversationClick = (conversationId: string, currentTitle: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setRenameConversationDialog({
      isOpen: true,
      conversationId,
      currentTitle,
    });
  };

  // 确认重命名对话
  const confirmRenameConversation = async (newTitle: string) => {
    const { conversationId } = renameConversationDialog;
    if (!conversationId) return;

    setIsRenamingConversation(true);
    try {
      await renameConversation(conversationId, newTitle);

      // 更新对话列表中的标题
      setConversations((prev) =>
        prev.map((c) => (c?.id === conversationId ? { ...c, title: newTitle } : c))
      );

      // 关闭对话框
      setRenameConversationDialog({ isOpen: false, conversationId: null, currentTitle: '' });
    } catch (error) {
      console.error('重命名对话失败:', error);
      alert('重命名对话失败');
    } finally {
      setIsRenamingConversation(false);
    }
  };

  // 取消重命名对话
  const cancelRenameConversation = () => {
    setRenameConversationDialog({ isOpen: false, conversationId: null, currentTitle: '' });
  };

  // 复制消息内容
  const handleCopyMessage = async (messageId: string, content: string) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedMessageId(messageId);
      // 2秒后恢复图标
      setTimeout(() => {
        setCopiedMessageId(null);
      }, 2000);
    } catch (error) {
      console.error('复制失败:', error);
      alert('复制失败，请重试');
    }
  };

  // 打开删除消息确认对话框
  const handleDeleteMessageClick = (messageId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!selectedConversationId) return;

    setDeleteMessageDialog({
      isOpen: true,
      messageId,
      conversationId: selectedConversationId,
    });
  };

  // 确认删除消息
  const confirmDeleteMessage = async () => {
    const { messageId, conversationId } = deleteMessageDialog;
    if (!messageId || !conversationId) return;

    setIsDeletingMessage(true);
    try {
      await deleteMessage(conversationId, messageId);

      // 从界面中移除消息
      setMessages((prev) => prev.filter((m) => m?.id !== messageId));

      // 更新对话列表中的消息数量
      setConversations((prev) =>
        prev.map((conv) =>
          conv?.id === conversationId
            ? { ...conv, message_count: Math.max(0, conv?.message_count - 1) }
            : conv
        )
      );

      // 关闭对话框
      setDeleteMessageDialog({ isOpen: false, messageId: null, conversationId: null });
    } catch (error) {
      console.error('删除消息失败:', error);
      alert('删除消息失败');
    } finally {
      setIsDeletingMessage(false);
    }
  };

  // 取消删除消息
  const cancelDeleteMessage = () => {
    setDeleteMessageDialog({ isOpen: false, messageId: null, conversationId: null });
  };

  // 打开清空消息确认对话框
  const handleClearMessagesClick = () => {
    if (!selectedConversationId) return;

    setClearMessagesDialog({
      isOpen: true,
      conversationId: selectedConversationId,
    });
  };

  // 确认清空消息
  const confirmClearMessages = async () => {
    const { conversationId } = clearMessagesDialog;
    if (!conversationId) return;

    setIsClearingMessages(true);
    try {
      await clearMessages(conversationId);

      // 清空界面中的消息
      setMessages([]);

      // 更新对话列表中的消息数量
      setConversations((prev) =>
        prev.map((conv) =>
          conv?.id === conversationId
            ? { ...conv, message_count: 0 }
            : conv
        )
      );

      // 关闭对话框
      setClearMessagesDialog({ isOpen: false, conversationId: null });
    } catch (error) {
      console.error('清空消息失败:', error);
      alert('清空消息失败');
    } finally {
      setIsClearingMessages(false);
    }
  };

  // 取消清空消息
  const cancelClearMessages = () => {
    setClearMessagesDialog({ isOpen: false, conversationId: null });
  };

  // 发送消息
  const handleSendMessage = async () => {
    if (!inputMessage.trim() || !selectedConversationId || isSending) return;

    const userMessage = inputMessage.trim();
    const currentConversationId = selectedConversationId;
    setInputMessage('');
    setIsSending(true);

    // 立即显示用户消息
    const tempUserMsg: Message = {
      id: `temp-user-${Date.now()}`,
      conversation_id: currentConversationId,
      role: 'user',
      content: userMessage,
      created_at: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, tempUserMsg]);
    setTimeout(scrollToBottom, 100);

    // 创建一个临时的 AI 消息用于流式更新
    const tempAiMsgId = `temp-ai-${Date.now()}`;
    const tempAiMsg: Message = {
      id: tempAiMsgId,
      conversation_id: currentConversationId,
      role: 'assistant',
      content: '',
      created_at: new Date().toISOString(),
    };

    try {
      // 添加空的 AI 消息
      setMessages((prev) => [...prev, tempAiMsg]);
      setTimeout(scrollToBottom, 100);

      // 调用流式 API
      await sendMessageStream(currentConversationId, userMessage, {
        onStart: () => {
          console.log('流式响应开始');
          setTempMessageSources([]);
        },
        onToken: (token: string) => {
          // 逐步更新 AI 消息内容
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === tempAiMsgId
                ? { ...msg, content: msg.content + token }
                : msg
            )
          );
          setTimeout(scrollToBottom, 50);
        },
        onContext: (sources: MessageSource[]) => {
          console.log('收到来源文档:', sources);
          setTempMessageSources(sources);
          // 默认展开来源文档
          setExpandedSources((prev) => {
            const newSet = new Set(prev);
            newSet.add(tempAiMsgId);
            return newSet;
          });
          // 立即更新临时消息的来源信息
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === tempAiMsgId
                ? { ...msg, sources }
                : msg
            )
          );
        },
        onEnd: (fullContent: string) => {
          console.log('流式响应结束，完整内容长度:', fullContent?.length);

          // 更新对话列表中的消息数量和更新时间
          setConversations((prev) => {
            const now = new Date().toISOString();
            const updated = prev.map((conv) =>
              conv.id === currentConversationId
                ? { 
                    ...conv, 
                    message_count: conv.message_count + 2,
                    updated_at: now,  // 更新时间为当前时间
                  }
                : conv
            );
            // 按 updated_at 降序排序（最新的在前面）
            return updated.sort((a, b) => {
              const timeA = a.updated_at || a.created_at;
              const timeB = b.updated_at || b.created_at;
              return timeB.localeCompare(timeA);
            });
          });

          // 重新加载消息列表以获取真实的消息ID
          loadMessages(currentConversationId).then(() => {
            // 使用 setTimeout 确保消息加载完成后再处理
            setTimeout(() => {
              setMessages((prevMessages) => {
                const lastAssistantMsg = [...prevMessages]
                  .reverse()
                  .find((m) => m?.role === 'assistant');

                if (lastAssistantMsg && tempMessageSources.length > 0) {
                  // sources 已经保存到后端数据库，从数据库加载的消息会包含 sources
                  // 只需迁移展开状态到真实 ID
                  requestAnimationFrame(() => {
                    setExpandedSources((prevExpanded) => {
                      const newSet = new Set(prevExpanded);
                      newSet.delete(tempAiMsgId); // 删除临时 ID
                      newSet.add(lastAssistantMsg.id); // 真实 ID 默认展开
                      console.log('展开状态已迁移:', lastAssistantMsg.id, '当前展开列表:', Array.from(newSet));
                      return newSet;
                    });
                  });
                }
                return prevMessages;
              });

              setIsSending(false);
              setTempMessageSources([]);
              setTimeout(scrollToBottom, 100);
              inputRef?.current?.focus();
            }, 50);
          });
        },
        onError: (error: string) => {
          console.error('流式响应错误:', error);
          // 移除临时 AI 消息
          setMessages((prev) => prev.filter((msg) => msg.id !== tempAiMsgId));
          setTempMessageSources([]);
          alert(`发送消息失败: ${error}`);
          setIsSending(false);
          inputRef?.current?.focus();
        },
      });
    } catch (error) {
      console.error('发送消息失败:', error);
      // 移除临时 AI 消息
      setMessages((prev) => prev.filter((msg) => msg.id !== tempAiMsgId));
      alert('发送消息失败，请重试');
      setIsSending(false);
      inputRef?.current?.focus();
    }
  };

  // 切换来源文档展开/收起
  const toggleSourcesExpanded = (messageId: string) => {
    setExpandedSources((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(messageId)) {
        newSet.delete(messageId);
      } else {
        newSet.add(messageId);
      }
      return newSet;
    });
  };

  // 格式化消息时间
  const formatMessageTime = (timestamp: string): string => {
    const msgDate = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - msgDate.getTime();
    const diffHours = diffMs / (1000 * 60 * 60);

    // 24小时内，只显示时间
    if (diffHours < 24) {
      return msgDate.toLocaleTimeString('zh-CN', {
        hour: '2-digit',
        minute: '2-digit',
      });
    }

    // 判断是否跨年
    const msgYear = msgDate.getFullYear();
    const nowYear = now.getFullYear();

    if (msgYear === nowYear) {
      // 同年，显示月日时间
      const month = String(msgDate.getMonth() + 1).padStart(2, '0');
      const day = String(msgDate.getDate()).padStart(2, '0');
      const time = msgDate.toLocaleTimeString('zh-CN', {
        hour: '2-digit',
        minute: '2-digit',
      });
      return `${month}-${day} ${time}`;
    } else {
      // 跨年，显示年月日时间
      const year = msgDate.getFullYear();
      const month = String(msgDate.getMonth() + 1).padStart(2, '0');
      const day = String(msgDate.getDate()).padStart(2, '0');
      const time = msgDate.toLocaleTimeString('zh-CN', {
        hour: '2-digit',
        minute: '2-digit',
      });
      return `${year}-${month}-${day} ${time}`;
    }
  };

  // 键盘事件处理
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  // 拖拽处理函数
  const handleMouseDown = () => {
    setIsResizing(true);
    // 拖拽时禁止文本选择
    document?.body?.classList?.add('no-select');
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!isResizing) return;

    const containerElement = document.querySelector('.chat-panel-container');
    if (!containerElement) return;

    const containerRect = containerElement?.getBoundingClientRect();
    const newWidth = e?.clientX - containerRect?.left;

    // 限制宽度范围：200px - 400px
    if (newWidth >= 200 && newWidth <= 400) {
      setConversationListWidth(newWidth);
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

  // 项目改变时重新加载对话
  useEffect(() => {
    if (projectId) {
      // 切换知识库时，清空当前选中的对话
      setSelectedConversationId(null);
      setMessages([]);
      // 加载新的对话列表
      loadConversations();
    } else {
      setConversations([]);
      setSelectedConversationId(null);
      setMessages([]);
    }
  }, [projectId]);

  // 持久化 expandedSources 到 localStorage
  useEffect(() => {
    try {
      const arr = Array.from(expandedSources);
      localStorage.setItem('expanded-sources', JSON.stringify(arr));
    } catch (error) {
      console.error('保存展开状态失败:', error);
    }
  }, [expandedSources]);

  // 选中对话改变时加载消息
  useEffect(() => {
    if (selectedConversationId) {
      loadMessages(selectedConversationId);
      // 不再清空展开状态，保持持久化的状态
    } else {
      setMessages([]);
    }
  }, [selectedConversationId]);

  if (!projectId) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <MessageCircle size={48} className="mx-auto mb-4 opacity-50" />
          <p>① 创建知识库 → ② 选择知识库 → ③ 进行问答</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex h-full chat-panel-container">
      {/* 对话列表侧边栏 */}
      {!isConversationListCollapsed && (
        <div
          className="bg-card border-r border-border flex flex-col"
          style={{ width: `${conversationListWidth}px` }}
        >
          <div className="p-4 border-b border-border dark:border-gray-900 flex items-center justify-between">
            <h3 className="text-base font-semibold text-foreground">对话列表</h3>
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setIsConversationListCollapsed(true)}
                title="收起对话列表"
              >
                <PanelLeftClose size={20} />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleCreateConversation}
                title="新建对话"
              >
                <Plus size={20} />
              </Button>
            </div>
          </div>

        <div className="flex-1 overflow-y-auto p-2">
          {isLoadingConversations ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 size={24} className="animate-spin text-primary" />
            </div>
          ) : conversations.length === 0 ? (
            <div className="text-center text-muted-foreground py-8 px-4">
              <MessageCircle size={32} className="mx-auto mb-2 opacity-50" />
              <p className="text-sm">暂无对话</p>
            </div>
          ) : (
            <div className="space-y-2">
              {conversations.map((conv) => (
                <Card
                  key={conv.id}
                  onClick={() => setSelectedConversationId(conv.id)}
                  className={`p-3 cursor-pointer transition-all group ${
                    selectedConversationId === conv.id
                      ? 'bg-accent border-2 border-primary'
                      : 'bg-secondary border-2 border-transparent hover:border-border hover:bg-accent'
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1 min-w-0">
                      <h4 className="font-medium text-card-foreground truncate text-sm">
                        {conv.title}
                      </h4>
                      <p className="text-xs text-muted-foreground mt-1">
                        {conv.message_count} 条消息
                      </p>
                    </div>
                    <div className="flex-shrink-0 ml-2">
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
                              handleRenameConversationClick(conv?.id, conv?.title, e as any);
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
                              handleDeleteConversationClick(conv?.id, e as any);
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
                </Card>
              ))}
            </div>
          )}
        </div>
        </div>
      )}

      {/* 拖拽分割线 */}
      {!isConversationListCollapsed && (
        <div
          className={`resizer ${isResizing ? 'resizing' : ''}`}
          onMouseDown={handleMouseDown}
        />
      )}

      {/* 聊天区域 */}
      <div className="flex-1 flex flex-col">
        {/* 头部 */}
        <div className="bg-card border-b border-border dark:border-gray-900 p-4">
          <div className="flex items-center gap-3">
            {isConversationListCollapsed && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setIsConversationListCollapsed(false)}
                title="展开对话列表"
              >
                <PanelLeftOpen size={20} />
              </Button>
            )}
            <div className="flex-1">
              <div className="flex items-center gap-2 text-foreground">
                <h2 className="text-base font-semibold">
                  {projectName || '知识库对话'}
                </h2>
                {selectedConversationId && (
                  <>
                    <span className="text-muted-foreground">/</span>
                    <span className="text-base text-muted-foreground">
                      {conversations.find((c) => c?.id === selectedConversationId)?.title || '对话'}
                    </span>
                  </>
                )}
              </div>
            </div>
            {selectedConversationId && messages?.length > 0 && (
              <Button
                variant="ghost"
                size="icon"
                onClick={handleClearMessagesClick}
                className="text-muted-foreground hover:text-destructive"
                title="清空所有消息"
              >
                <Eraser size={20} />
              </Button>
            )}
          </div>
        </div>

        {/* 消息列表 */}
        <div className="flex-1 overflow-y-auto p-4 bg-secondary">
          {!selectedConversationId ? (
            <div className="h-full flex items-center justify-center text-muted-foreground">
              <div className="text-center">
                <MessageCircle size={64} className="mx-auto mb-4 opacity-50" />
                <p className="mb-6 text-lg">暂无对话</p>
                <Button
                  onClick={handleCreateConversation}
                  size="lg"
                  className="text-base font-medium"
                >
                  <Plus size={18} className="mr-1.5" />
                  新建对话
                </Button>
              </div>
            </div>
          ) : isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 size={32} className="animate-spin text-primary" />
            </div>
          ) : messages.length === 0 ? (
            <div className="h-full flex items-center justify-center text-muted-foreground">
              <div className="text-center">
                <MessageCircle size={48} className="mx-auto mb-4 opacity-50" />
                <p>开始对话吧！</p>
                <p className="text-sm mt-2">基于当前知识库内容回答您的问题</p>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              {messages.map((msg) => (
                <div
                  key={msg.id}
                  className={`flex flex-col group ${msg.role === 'user' ? 'items-end' : 'items-start'}`}
                >
                  <div
                    className={`rounded-lg px-4 py-3 ${
                      msg.role === 'user'
                        ? 'max-w-[70%] bg-primary text-primary-foreground'
                        : 'max-w-[85%] bg-card border border-border dark:border-gray-900 text-foreground'
                    }`}
                  >
                    {msg?.role === 'assistant' ? (
                      <div className="prose prose-sm dark:prose-invert max-w-none">
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code(props) {
                              const { children, className, ...rest } = props;
                              const match = /language-(\w+)/.exec(className || '');
                              return match ? (
                                <SyntaxHighlighter
                                  style={vscDarkPlus as any}
                                  language={match[1]}
                                  PreTag="div"
                                >
                                  {String(children).replace(/\n$/, '')}
                                </SyntaxHighlighter>
                              ) : (
                                <code className={className} {...rest}>
                                  {children}
                                </code>
                              );
                            },
                            a(props) {
                              const { href, children, ...rest } = props;
                              return (
                                <a
                                  href={href}
                                  onClick={(e) => {
                                    e.preventDefault();
                                    if (href) {
                                      open(href).catch((err) => {
                                        console.error('打开链接失败:', err);
                                        alert(`打开链接失败: ${err}`);
                                      });
                                    }
                                  }}
                                  className="text-blue-500 hover:text-blue-600 underline cursor-pointer"
                                  {...rest}
                                >
                                  {children}
                                </a>
                              );
                            },
                          }}
                        >
                          {msg?.content}
                        </ReactMarkdown>
                        {isSending && msg?.id?.startsWith('temp-ai-') && (
                          <div className="flex items-start">
                            <div className="max-w-[70%] bg-card px-4 py-3">
                                <Loader2 size={20} className="animate-spin text-primary" />
                            </div>
                          </div>
                        )}
                      </div>
                    ) : (
                      <div className="text-sm whitespace-pre-wrap break-words">{msg?.content}</div>
                    )}
                    {/* 显示来源文档 */}
                    {msg?.role === 'assistant' && msg?.sources && msg?.sources?.length > 0 && (
                      <div className="mt-3 pt-3 border-t border-border/50 dark:border-gray-900/50">
                        <button
                          onClick={() => toggleSourcesExpanded(msg.id)}
                          className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors"
                        >
                          {expandedSources.has(msg.id) ? (
                            <ChevronDown size={14} className="flex-shrink-0" />
                          ) : (
                            <ChevronRight size={14} className="flex-shrink-0" />
                          )}
                          <FileText size={14} className="flex-shrink-0" />
                          <span className="font-medium">
                            来源文档 ({msg?.sources?.length})
                          </span>
                        </button>

                        {expandedSources.has(msg.id) && (
                          <div className="mt-2 space-y-1">
                            {msg?.sources?.map((source, idx) => (
                              <div
                                key={idx}
                                className="flex items-center justify-between text-xs bg-secondary/50 px-3 py-2 rounded hover:bg-secondary/70 transition-colors group"
                                title={source?.filename}
                              >
                                <div className="flex items-center gap-2 flex-1 min-w-0">
                                  <FileText size={12} className="text-muted-foreground flex-shrink-0" />
                                  <span className="text-foreground/80 font-mono truncate">
                                    {source?.filename}
                                  </span>
                                </div>
                                <span className="text-muted-foreground ml-2 flex-shrink-0 font-medium">
                                  {(source?.relevance_score * 100)?.toFixed(0)}%
                                </span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                  <div className="flex items-center gap-2 mt-1 px-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleCopyMessage(msg?.id, msg?.content)}
                      className="h-auto p-1 text-muted-foreground hover:text-foreground"
                      title={copiedMessageId === msg?.id ? "已复制" : "复制消息"}
                    >
                      {copiedMessageId === msg?.id ? (
                        <Check size={12} className="text-green-500" />
                      ) : (
                        <Copy size={12} />
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => handleDeleteMessageClick(msg?.id, e)}
                      className="h-auto p-1 text-muted-foreground hover:text-destructive"
                      title="删除消息"
                    >
                      <Trash2 size={12} />
                    </Button>
                    <span className="text-xs text-muted-foreground">
                      {formatMessageTime(msg?.created_at)}
                    </span>
                  </div>
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>

        {/* 输入框 */}
        <div className="bg-card border-t border-border p-4">
          {selectedConversationId ? (
            <div className="flex items-start gap-3">
              <Textarea
                ref={inputRef}
                value={inputMessage}
                onChange={(e) => setInputMessage(e?.target?.value)}
                onKeyDown={handleKeyDown}
                placeholder="输入消息... (Shift+Enter 换行，Enter 发送)"
                className="flex-1 rounded-lg resize-none min-h-[80px]"
                disabled={isSending}
              />
              <div className="flex flex-col gap-2">
                <Button
                  size="icon"
                  onClick={handleSendMessage}
                  disabled={!inputMessage?.trim() || isSending}
                  title={isSending ? "发送中..." : "发送消息"}
                >
                  <Send size={16} />
                </Button>
                {isVoiceSupported && (
                  <Button
                    size="icon"
                    variant={isRecording ? "destructive" : isRecognizing ? "default" : "secondary"}
                    onClick={toggleRecording}
                    disabled={isSending || isRecognizing}
                    className={isRecording ? "animate-pulse" : ""}
                    title={
                      isRecording
                        ? '点击停止录音'
                        : isRecognizing
                        ? '识别中...'
                        : '语音输入'
                    }
                  >
                    {isRecognizing ? (
                      <Loader2 size={16} className="animate-spin" />
                    ) : isRecording ? (
                      <MicOff size={16} />
                    ) : (
                      <Mic size={16} />
                    )}
                  </Button>
                )}
              </div>
            </div>
          ) : (
            <div className="text-center text-muted-foreground py-4">
              请选择或创建一个对话
            </div>
          )}
        </div>
      </div>

      {/* 删除消息确认对话框 */}
      <ConfirmDialog
        isOpen={deleteMessageDialog.isOpen}
        onClose={cancelDeleteMessage}
        onConfirm={confirmDeleteMessage}
        title="删除消息"
        message="确定要删除这条消息吗？此操作无法撤销。"
        confirmText="删除"
        cancelText="取消"
        type="danger"
        isLoading={isDeletingMessage}
      />

      {/* 删除对话确认对话框 */}
      <ConfirmDialog
        isOpen={deleteConversationDialog.isOpen}
        onClose={cancelDeleteConversation}
        onConfirm={confirmDeleteConversation}
        title="删除对话"
        message="确定要删除这个对话吗？对话中的所有消息都会被删除，此操作无法撤销。"
        confirmText="删除"
        cancelText="取消"
        type="danger"
        isLoading={isDeletingConversation}
      />

      {/* 清空消息确认对话框 */}
      <ConfirmDialog
        isOpen={clearMessagesDialog.isOpen}
        onClose={cancelClearMessages}
        onConfirm={confirmClearMessages}
        title="清空所有消息"
        message="确定要清空当前对话的所有消息吗？此操作无法撤销。"
        confirmText="清空"
        cancelText="取消"
        type="danger"
        isLoading={isClearingMessages}
      />

      {/* 重命名对话对话框 */}
      <RenameDialog
        isOpen={renameConversationDialog.isOpen}
        currentName={renameConversationDialog.currentTitle}
        onClose={cancelRenameConversation}
        onConfirm={confirmRenameConversation}
        title="重命名对话"
        description="请输入对话的新名称"
        isLoading={isRenamingConversation}
      />
    </div>
  );
};

export default ChatPanel;

