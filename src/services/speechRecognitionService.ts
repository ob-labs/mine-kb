import { invoke } from '@tauri-apps/api/tauri';

/**
 * 将 Blob 转换为 Base64
 */
async function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onloadend = () => {
      const base64 = reader?.result as string;
      // 移除 data:audio/...;base64, 前缀
      const base64Data = base64?.split(',')[1] || '';
      resolve(base64Data);
    };
    reader.onerror = reject;
    reader.readAsDataURL(blob);
  });
}

/**
 * 识别语音（调用 Tauri 后端）
 */
export async function recognizeSpeech(audioBlob: Blob): Promise<string> {
  try {
    console.log('开始语音识别...');
    console.log('音频大小:', audioBlob.size, 'bytes');
    console.log('音频类型:', audioBlob.type);

    // 转换为 Base64
    const base64Audio = await blobToBase64(audioBlob);
    console.log('Base64 编码完成，长度:', base64Audio.length);

    // 调用 Tauri 后端命令
    const result = await invoke<string>('recognize_speech', {
      audioData: base64Audio,
      audioFormat: audioBlob.type || 'audio/webm',
    });

    console.log('识别结果:', result);
    return result;
  } catch (error) {
    console.error('语音识别失败:', error);

    // 解析错误信息
    let errorMessage = '语音识别失败';
    if (typeof error === 'string') {
      errorMessage = error;
    } else if (error instanceof Error) {
      errorMessage = error.message;
    }

    throw new Error(errorMessage);
  }
}

/**
 * 检查语音识别服务配置
 */
export async function checkSpeechConfig(): Promise<{
  configured: boolean;
  provider?: string;
  message?: string;
}> {
  try {
    const result = await invoke<{ configured: boolean; provider?: string; message?: string }>(
      'check_speech_config'
    );
    return result;
  } catch (error) {
    console.error('检查语音配置失败:', error);
    return {
      configured: false,
      message: '无法检查配置',
    };
  }
}

