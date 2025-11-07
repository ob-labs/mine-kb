import React, { useState } from 'react';

/**
 * 语音识别诊断组件
 * 用于测试Tauri环境中的Web Speech API兼容性
 */
const VoiceInputTest: React.FC = () => {
  const [logs, setLogs] = useState<string[]>([]);
  const [testStatus, setTestStatus] = useState<'idle' | 'testing' | 'success' | 'failed'>('idle');

  const addLog = (message: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${message}`]);
    console.log(message);
  };

  const testWebSpeechAPI = async () => {
    setLogs([]);
    setTestStatus('testing');
    addLog('开始测试...');

    try {
      // 测试1: 检查API是否存在
      addLog('1. 检查Web Speech API是否存在');
      const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;

      if (!SpeechRecognition) {
        addLog('❌ Web Speech API 不存在');
        setTestStatus('failed');
        return;
      }
      addLog('✅ Web Speech API 存在');

      // 测试2: 检查MediaDevices API
      addLog('2. 检查MediaDevices API');
      if (!navigator?.mediaDevices || !navigator?.mediaDevices?.getUserMedia) {
        addLog('❌ MediaDevices API 不可用');
        setTestStatus('failed');
        return;
      }
      addLog('✅ MediaDevices API 可用');

      // 测试3: 请求麦克风权限
      addLog('3. 请求麦克风权限...');
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        addLog('✅ 麦克风权限已授予');
        stream.getTracks().forEach((track) => track.stop());
      } catch (error: any) {
        addLog(`❌ 麦克风权限失败: ${error.message}`);
        setTestStatus('failed');
        return;
      }

      // 测试4: 创建SpeechRecognition实例
      addLog('4. 创建SpeechRecognition实例...');
      let recognition: any;
      try {
        recognition = new SpeechRecognition();
        addLog('✅ 实例创建成功');
      } catch (error: any) {
        addLog(`❌ 实例创建失败: ${error.message}`);
        setTestStatus('failed');
        return;
      }

      // 测试5: 配置recognition
      addLog('5. 配置recognition...');
      try {
        recognition.lang = 'zh-CN';
        recognition.continuous = false;
        recognition.interimResults = false;
        addLog('✅ 配置成功');
      } catch (error: any) {
        addLog(`❌ 配置失败: ${error.message}`);
        setTestStatus('failed');
        return;
      }

      // 测试6: 尝试启动（这是最容易崩溃的地方）
      addLog('6. 尝试启动recognition...');
      addLog('⚠️  注意：如果应用在这一步崩溃，说明WKWebView不支持Web Speech API');

      return new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => {
          addLog('❌ 启动超时（10秒）');
          try {
            recognition?.abort();
          } catch (e) {
            // ignore
          }
          setTestStatus('failed');
          reject(new Error('超时'));
        }, 10000);

        recognition.onstart = () => {
          clearTimeout(timeout);
          addLog('✅ recognition.onstart 触发！');
          addLog('🎉 Web Speech API 完全可用！');
          setTimeout(() => {
            try {
              recognition.stop();
            } catch (e) {
              addLog(`停止时出错: ${e}`);
            }
          }, 1000);
        };

        recognition.onend = () => {
          addLog('✅ recognition.onend 触发');
          setTestStatus('success');
          resolve();
        };

        recognition.onerror = (event: any) => {
          clearTimeout(timeout);
          addLog(`❌ recognition.onerror: ${event.error}`);
          setTestStatus('failed');
          reject(new Error(event.error));
        };

        try {
          recognition.start();
          addLog('⏳ start() 调用完成，等待onstart事件...');
        } catch (error: any) {
          clearTimeout(timeout);
          addLog(`❌ start() 抛出异常: ${error.message}`);
          setTestStatus('failed');
          reject(error);
        }
      });
    } catch (error: any) {
      addLog(`❌ 测试失败: ${error.message}`);
      setTestStatus('failed');
    }
  };

  const testMediaRecorder = async () => {
    setLogs([]);
    setTestStatus('testing');
    addLog('开始测试MediaRecorder（录音API）...');

    try {
      // 请求麦克风
      addLog('1. 请求麦克风权限...');
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      addLog('✅ 麦克风权限已授予');

      // 测试MediaRecorder
      addLog('2. 测试MediaRecorder API...');
      if (!window.MediaRecorder) {
        addLog('❌ MediaRecorder 不可用');
        setTestStatus('failed');
        return;
      }

      const recorder = new MediaRecorder(stream);
      addLog('✅ MediaRecorder 创建成功');

      recorder.ondataavailable = (e) => {
        addLog(`✅ 录音数据大小: ${e.data.size} bytes`);
      };

      recorder.onstart = () => {
        addLog('✅ 录音开始');
      };

      recorder.onstop = () => {
        addLog('✅ 录音停止');
        stream.getTracks().forEach((track) => track.stop());
        setTestStatus('success');
        addLog('🎉 MediaRecorder 完全可用！可以用它实现录音功能');
      };

      recorder.start();
      addLog('⏳ 录音中... 2秒后停止');

      setTimeout(() => {
        recorder.stop();
      }, 2000);
    } catch (error: any) {
      addLog(`❌ 测试失败: ${error.message}`);
      setTestStatus('failed');
    }
  };

  return (
    <div className="p-6 bg-card rounded-lg border border-border max-w-2xl mx-auto mt-8">
      <h2 className="text-xl font-bold mb-4">语音输入功能诊断</h2>

      <div className="space-y-3 mb-4">
        <button
          onClick={testWebSpeechAPI}
          disabled={testStatus === 'testing'}
          className="w-full px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90 disabled:opacity-50"
        >
          测试 Web Speech API（语音识别）
        </button>

        <button
          onClick={testMediaRecorder}
          disabled={testStatus === 'testing'}
          className="w-full px-4 py-2 bg-secondary text-foreground rounded-lg hover:bg-accent disabled:opacity-50"
        >
          测试 MediaRecorder（录音API）
        </button>
      </div>

      <div className="bg-background p-4 rounded border border-border">
        <div className="flex items-center justify-between mb-2">
          <h3 className="font-semibold">测试日志</h3>
          {testStatus !== 'idle' && (
            <span
              className={`px-2 py-1 rounded text-xs ${
                testStatus === 'testing'
                  ? 'bg-blue-100 text-blue-700'
                  : testStatus === 'success'
                  ? 'bg-green-100 text-green-700'
                  : 'bg-red-100 text-red-700'
              }`}
            >
              {testStatus === 'testing'
                ? '测试中...'
                : testStatus === 'success'
                ? '测试成功'
                : '测试失败'}
            </span>
          )}
        </div>

        <div className="space-y-1 max-h-96 overflow-y-auto font-mono text-sm">
          {logs.length === 0 ? (
            <p className="text-muted-foreground">点击上方按钮开始测试</p>
          ) : (
            logs.map((log, index) => (
              <div
                key={index}
                className={`${
                  log.includes('❌')
                    ? 'text-red-600'
                    : log.includes('✅')
                    ? 'text-green-600'
                    : log.includes('⚠️')
                    ? 'text-yellow-600'
                    : log.includes('🎉')
                    ? 'text-blue-600 font-bold'
                    : 'text-foreground'
                }`}
              >
                {log}
              </div>
            ))
          )}
        </div>
      </div>

      <div className="mt-4 text-sm text-muted-foreground">
        <p className="font-semibold mb-2">说明：</p>
        <ul className="list-disc list-inside space-y-1">
          <li>如果应用在测试时崩溃，说明当前环境不支持该API</li>
          <li>Web Speech API需要网络连接（使用Google服务）</li>
          <li>MediaRecorder可以在本地录音，需要配合语音识别服务使用</li>
        </ul>
      </div>
    </div>
  );
};

export default VoiceInputTest;

