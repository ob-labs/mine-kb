import { useState, useEffect, useRef, useCallback } from 'react';

// 语音识别接口类型定义
interface SpeechRecognitionEvent extends Event {
  results: SpeechRecognitionResultList;
  resultIndex: number;
}

interface SpeechRecognitionErrorEvent extends Event {
  error: string;
  message: string;
}

interface SpeechRecognition extends EventTarget {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  start(): void;
  stop(): void;
  abort(): void;
  onstart: ((this: SpeechRecognition, ev: Event) => any) | null;
  onend: ((this: SpeechRecognition, ev: Event) => any) | null;
  onresult: ((this: SpeechRecognition, ev: SpeechRecognitionEvent) => any) | null;
  onerror: ((this: SpeechRecognition, ev: SpeechRecognitionErrorEvent) => any) | null;
}

// 声明全局接口
declare global {
  interface Window {
    SpeechRecognition: new () => SpeechRecognition;
    webkitSpeechRecognition: new () => SpeechRecognition;
  }
}

interface UseSpeechRecognitionOptions {
  onTranscript?: (transcript: string) => void;
  onError?: (error: string) => void;
  lang?: string;
  continuous?: boolean;
  interimResults?: boolean;
}

export const useSpeechRecognition = (options: UseSpeechRecognitionOptions = {}) => {
  const {
    onTranscript,
    onError,
    lang = 'zh-CN',
    continuous = true,
    interimResults = true,
  } = options;

  const [isRecording, setIsRecording] = useState(false);
  const [isSupported, setIsSupported] = useState(false);
  const recognitionRef = useRef<SpeechRecognition | null>(null);

  // 检查浏览器支持
  useEffect(() => {
    try {
      const SpeechRecognition = window?.SpeechRecognition || window?.webkitSpeechRecognition;
      setIsSupported(!!SpeechRecognition);

      if (SpeechRecognition) {
        try {
          recognitionRef.current = new SpeechRecognition();
          const recognition = recognitionRef.current;

          recognition.continuous = continuous;
          recognition.interimResults = interimResults;
          recognition.lang = lang;

          recognition.onstart = () => {
            console.log('语音识别已启动');
            setIsRecording(true);
          };

          recognition.onend = () => {
            console.log('语音识别已结束');
            setIsRecording(false);
          };

          recognition.onresult = (event: SpeechRecognitionEvent) => {
            try {
              let finalTranscript = '';
              let interimTranscript = '';

              for (let i = event?.resultIndex; i < event?.results?.length; i++) {
                const transcript = event?.results?.[i]?.[0]?.transcript;
                if (event?.results?.[i]?.isFinal) {
                  finalTranscript += transcript;
                } else {
                  interimTranscript += transcript;
                }
              }

              // 优先使用最终结果，否则使用临时结果
              const textToSend = finalTranscript || interimTranscript;
              if (textToSend && onTranscript) {
                onTranscript(textToSend);
              }
            } catch (e) {
              console.error('处理识别结果失败:', e);
            }
          };

          recognition.onerror = (event: SpeechRecognitionErrorEvent) => {
            console.error('语音识别错误:', event?.error);
            setIsRecording(false);

            let errorMessage = '语音识别失败';
            switch (event?.error) {
              case 'not-allowed':
              case 'permission-denied':
                errorMessage = '请允许麦克风权限';
                break;
              case 'no-speech':
                errorMessage = '未检测到语音';
                break;
              case 'network':
                errorMessage = '网络错误，请检查网络连接';
                break;
              case 'aborted':
                errorMessage = '语音识别已取消';
                break;
              default:
                errorMessage = `语音识别错误: ${event?.error}`;
            }

            if (onError) {
              onError(errorMessage);
            }
          };

          console.log('语音识别初始化成功');
        } catch (e) {
          console.error('创建语音识别实例失败:', e);
          setIsSupported(false);
        }
      }
    } catch (e) {
      console.error('初始化语音识别失败:', e);
      setIsSupported(false);
    }

    return () => {
      if (recognitionRef?.current) {
        try {
          recognitionRef.current.abort();
        } catch (e) {
          console.error('停止语音识别失败:', e);
        }
      }
    };
  }, [lang, continuous, interimResults, onTranscript, onError]);

  // 开始录音
  const startRecording = useCallback(async () => {
    if (!recognitionRef?.current || isRecording) return;

    try {
      console.log('尝试启动语音识别...');

      // 先请求麦克风权限（防止崩溃）
      if (navigator?.mediaDevices && navigator?.mediaDevices?.getUserMedia) {
        try {
          console.log('请求麦克风权限...');
          const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
          console.log('麦克风权限已授予');
          // 立即关闭stream，我们只需要权限
          stream?.getTracks()?.forEach(track => track?.stop());
        } catch (permError) {
          console.error('麦克风权限被拒绝:', permError);
          if (onError) {
            onError('请允许访问麦克风');
          }
          return;
        }
      }

      // 启动语音识别
      recognitionRef.current.start();
      console.log('语音识别启动成功');
    } catch (error) {
      console.error('启动语音识别失败:', error);
      const errorMsg = error instanceof Error ? error.message : String(error);
      if (onError) {
        onError(`启动失败: ${errorMsg}`);
      }
    }
  }, [isRecording, onError]);

  // 停止录音
  const stopRecording = useCallback(() => {
    if (!recognitionRef?.current || !isRecording) return;

    try {
      recognitionRef.current.stop();
    } catch (error) {
      console.error('停止语音识别失败:', error);
    }
  }, [isRecording]);

  // 切换录音状态
  const toggleRecording = useCallback(() => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  }, [isRecording, startRecording, stopRecording]);

  return {
    isRecording,
    isSupported,
    startRecording,
    stopRecording,
    toggleRecording,
  };
};

