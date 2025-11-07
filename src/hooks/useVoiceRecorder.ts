import { useState, useRef, useCallback } from 'react';

interface UseVoiceRecorderOptions {
  onRecordingComplete?: (audioBlob: Blob) => void;
  onError?: (error: string) => void;
}

export const useVoiceRecorder = (options: UseVoiceRecorderOptions = {}) => {
  const { onRecordingComplete, onError } = options;

  const [isRecording, setIsRecording] = useState(false);
  const [isSupported, setIsSupported] = useState(true);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  // 检查浏览器支持
  const checkSupport = useCallback(() => {
    const supported = !!(
      navigator?.mediaDevices &&
      // @ts-ignore - getUserMedia may not be available in all browsers
      navigator?.mediaDevices?.getUserMedia &&
      'MediaRecorder' in window
    );
    setIsSupported(supported);
    return supported;
  }, []);

  // 开始录音
  const startRecording = useCallback(async () => {
    if (!checkSupport()) {
      const error = '浏览器不支持录音功能';
      console.error(error);
      if (onError) {
        onError(error);
      }
      return;
    }

    if (isRecording) {
      console.warn('已在录音中');
      return;
    }

    try {
      console.log('请求麦克风权限...');

      // 请求麦克风权限
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        }
      });

      console.log('麦克风权限已授予');
      streamRef.current = stream;
      chunksRef.current = [];

      // 创建 MediaRecorder
      const mimeType = MediaRecorder.isTypeSupported('audio/webm')
        ? 'audio/webm'
        : MediaRecorder.isTypeSupported('audio/mp4')
        ? 'audio/mp4'
        : 'audio/wav';

      console.log(`使用音频格式: ${mimeType}`);

      const mediaRecorder = new MediaRecorder(stream, {
        mimeType,
      });
      mediaRecorderRef.current = mediaRecorder;

      // 监听数据
      mediaRecorder.ondataavailable = (event) => {
        if (event?.data && event?.data?.size > 0) {
          chunksRef.current.push(event.data);
          console.log(`收到音频数据: ${event.data.size} bytes`);
        }
      };

      // 录音停止
      mediaRecorder.onstop = () => {
        console.log('录音已停止');

        // 合并所有音频块
        const audioBlob = new Blob(chunksRef.current, { type: mimeType });
        console.log(`音频总大小: ${audioBlob.size} bytes`);

        // 清理
        if (streamRef?.current) {
          streamRef.current.getTracks().forEach((track) => track?.stop());
          streamRef.current = null;
        }

        // 回调
        if (onRecordingComplete && audioBlob.size > 0) {
          onRecordingComplete(audioBlob);
        } else if (audioBlob.size === 0 && onError) {
          onError('录音数据为空');
        }

        setIsRecording(false);
        chunksRef.current = [];
      };

      // 错误处理
      mediaRecorder.onerror = (event: Event) => {
        console.error('录音错误:', event);
        const error = '录音过程中发生错误';
        if (onError) {
          onError(error);
        }
        stopRecording();
      };

      // 开始录音
      mediaRecorder.start(100); // 每100ms收集一次数据
      setIsRecording(true);
      console.log('开始录音...');
    } catch (error) {
      console.error('启动录音失败:', error);
      let errorMessage = '启动录音失败';

      if (error instanceof Error) {
        if (error?.name === 'NotAllowedError' || error?.name === 'PermissionDeniedError') {
          errorMessage = '请允许访问麦克风';
        } else if (error?.name === 'NotFoundError') {
          errorMessage = '未找到麦克风设备';
        } else {
          errorMessage = `启动录音失败: ${error.message}`;
        }
      }

      if (onError) {
        onError(errorMessage);
      }
      setIsRecording(false);
    }
  }, [isRecording, checkSupport, onRecordingComplete, onError]);

  // 停止录音
  const stopRecording = useCallback(() => {
    if (!isRecording || !mediaRecorderRef?.current) {
      return;
    }

    try {
      if (mediaRecorderRef.current.state !== 'inactive') {
        mediaRecorderRef.current.stop();
        console.log('正在停止录音...');
      }
    } catch (error) {
      console.error('停止录音失败:', error);
      if (onError) {
        onError('停止录音失败');
      }
    }
  }, [isRecording, onError]);

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

