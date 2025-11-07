import { invoke } from '@tauri-apps/api/tauri';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

// ==================== 类型定义 ====================

export interface Conversation {
  id: string;
  project_id: string;
  title: string;
  created_at: string;
  updated_at?: string;  // 添加 updated_at 字段用于排序
  message_count: number;
}

export interface MessageSource {
  filename: string;
  relevance_score: number;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  created_at: string;
  sources?: MessageSource[];
}

export interface CreateConversationRequest {
  project_id: string;
  title?: string;
}

export interface SendMessageRequest {
  conversation_id: string;
  content: string;
}

export interface DeleteConversationRequest {
  conversation_id: string;
}

export interface DeleteMessageRequest {
  conversation_id: string;
  message_id: string;
}

export interface RenameConversationRequest {
  conversation_id: string;
  new_title: string;
}

export interface StreamCallbacks {
  onStart?: () => void;
  onToken: (token: string) => void;
  onContext?: (sources: MessageSource[]) => void;
  onEnd?: (fullContent: string) => void;
  onError?: (error: string) => void;
}

// ==================== API 函数 ====================

/**
 * 创建新对话
 */
export async function createConversation(
  projectId: string,
  title?: string
): Promise<Conversation> {
  try {
    const request: CreateConversationRequest = {
      project_id: projectId,
      title: title,
    };
    const conversation = await invoke<Conversation>('create_conversation', { request });
    return conversation;
  } catch (error) {
    console.error('创建对话失败:', error);
    throw new Error(`创建对话失败: ${error}`);
  }
}

/**
 * 获取项目的所有对话列表
 */
export async function getConversations(projectId: string): Promise<Conversation[]> {
  try {
    const conversations = await invoke<Conversation[]>('get_conversations', { projectId });
    return conversations;
  } catch (error) {
    console.error('获取对话列表失败:', error);
    throw new Error(`获取对话列表失败: ${error}`);
  }
}

/**
 * 获取对话的历史消息
 */
export async function getConversationHistory(conversationId: string): Promise<Message[]> {
  try {
    const messages = await invoke<Message[]>('get_conversation_history', { conversationId });
    return messages;
  } catch (error) {
    console.error('获取对话历史失败:', error);
    throw new Error(`获取对话历史失败: ${error}`);
  }
}

/**
 * 发送消息（流式版本）
 */
export async function sendMessageStream(
  conversationId: string,
  content: string,
  callbacks: StreamCallbacks
): Promise<void> {
  const unlistenFns: UnlistenFn[] = [];

  try {
    // 监听流式开始事件
    const unlistenStart = await listen<string>('chat-stream-start', (event) => {
      if (event?.payload === conversationId) {
        callbacks?.onStart?.();
      }
    });
    unlistenFns.push(unlistenStart);

    // 监听流式 token 事件
    const unlistenToken = await listen<{ conversation_id: string; token: string }>(
      'chat-stream-token',
      (event) => {
        if (event?.payload?.conversation_id === conversationId) {
          callbacks.onToken(event?.payload?.token || '');
        }
      }
    );
    unlistenFns.push(unlistenToken);

    // 监听来源文档事件
    const unlistenContext = await listen<{ conversation_id: string; sources: MessageSource[] }>(
      'chat-stream-context',
      (event) => {
        if (event?.payload?.conversation_id === conversationId) {
          callbacks?.onContext?.(event?.payload?.sources || []);
        }
      }
    );
    unlistenFns.push(unlistenContext);

    // 监听流式结束事件
    const unlistenEnd = await listen<{ conversation_id: string; content: string }>(
      'chat-stream-end',
      (event) => {
        if (event?.payload?.conversation_id === conversationId) {
          callbacks?.onEnd?.(event?.payload?.content || '');
          // 清理监听器
          unlistenFns.forEach((fn) => fn());
        }
      }
    );
    unlistenFns.push(unlistenEnd);

    // 监听错误事件
    const unlistenError = await listen<{ conversation_id: string; error: string }>(
      'chat-stream-error',
      (event) => {
        if (event?.payload?.conversation_id === conversationId) {
          callbacks?.onError?.(event?.payload?.error || '未知错误');
          // 清理监听器
          unlistenFns.forEach((fn) => fn());
        }
      }
    );
    unlistenFns.push(unlistenError);

    // 发送消息请求
    const request: SendMessageRequest = {
      conversation_id: conversationId,
      content,
    };
    await invoke<string>('send_message', { request });
  } catch (error) {
    console.error('发送消息失败:', error);
    // 清理监听器
    unlistenFns.forEach((fn) => fn());
    callbacks?.onError?.(String(error));
    throw new Error(`发送消息失败: ${error}`);
  }
}

/**
 * 发送消息（同步版本，等待完整响应）- 保留用于兼容性
 */
export async function sendMessage(
  conversationId: string,
  content: string
): Promise<string> {
  try {
    const request: SendMessageRequest = {
      conversation_id: conversationId,
      content,
    };
    const response = await invoke<string>('send_message', { request });
    return response;
  } catch (error) {
    console.error('发送消息失败:', error);
    throw new Error(`发送消息失败: ${error}`);
  }
}

/**
 * 删除对话
 */
export async function deleteConversation(conversationId: string): Promise<boolean> {
  try {
    const request: DeleteConversationRequest = {
      conversation_id: conversationId,
    };
    const result = await invoke<boolean>('delete_conversation', { request });
    return result;
  } catch (error) {
    console.error('删除对话失败:', error);
    throw new Error(`删除对话失败: ${error}`);
  }
}

/**
 * 删除单条消息
 */
export async function deleteMessage(
  conversationId: string,
  messageId: string
): Promise<boolean> {
  try {
    const request: DeleteMessageRequest = {
      conversation_id: conversationId,
      message_id: messageId,
    };
    const result = await invoke<boolean>('delete_message', { request });
    return result;
  } catch (error) {
    console.error('删除消息失败:', error);
    throw new Error(`删除消息失败: ${error}`);
  }
}

/**
 * 清空对话的所有消息
 */
export async function clearMessages(conversationId: string): Promise<boolean> {
  try {
    const request = {
      conversation_id: conversationId,
    };
    const result = await invoke<boolean>('clear_messages', { request });
    return result;
  } catch (error) {
    console.error('清空消息失败:', error);
    throw new Error(`清空消息失败: ${error}`);
  }
}

/**
 * 重命名对话
 */
export async function renameConversation(
  conversationId: string,
  newTitle: string
): Promise<boolean> {
  try {
    const request: RenameConversationRequest = {
      conversation_id: conversationId,
      new_title: newTitle,
    };
    const result = await invoke<boolean>('rename_conversation', { request });
    return result;
  } catch (error) {
    console.error('重命名对话失败:', error);
    throw new Error(`重命名对话失败: ${error}`);
  }
}

// ==================== 辅助函数 ====================

/**
 * 格式化时间显示
 */
export function formatMessageTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);

    if (diffMins < 1) return '刚刚';
    if (diffMins < 60) return `${diffMins}分钟前`;
    if (diffHours < 24) return `${diffHours}小时前`;

    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return isoString;
  }
}

/**
 * 获取对话标题（如果为空则返回默认标题）
 */
export function getConversationTitle(conversation: Conversation): string {
  return conversation.title || '新对话';
}

