import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import Layout from "./components/Layout/Layout";
import SplashScreen from "./components/common/SplashScreen";
import { useTheme } from "./hooks/useTheme";

interface StartupEvent {
  step: number;
  total_steps: number;
  message: string;
  status: 'progress' | 'success' | 'error';
  details?: string;
  error?: string;
}

function App() {
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const { theme, toggleTheme } = useTheme();
  
  // Startup state
  const [isStarting, setIsStarting] = useState(true);
  const [startupProgress, setStartupProgress] = useState<StartupEvent>({
    step: 0,
    total_steps: 3,
    message: "正在启动应用...",
    status: "progress",
    details: "首次启动需要初始化环境，可能需要几分钟，请耐心等待..."
  });

  useEffect(() => {
    // Listen for startup progress events
    const unlistenPromise = listen<StartupEvent>('startup-progress', (event) => {
      const payload = event.payload;
      setStartupProgress(payload);
      
      // If startup is complete, hide splash screen after a short delay
      if (payload.status === 'success' && payload.step === payload.total_steps) {
        setTimeout(() => {
          setIsStarting(false);
        }, 800);
      }
    });

    // 设置启动超时检测（5分钟）
    const startupTimeout = setTimeout(() => {
      // 如果5分钟后还在步骤0或1，显示超时提示
      setStartupProgress((prev) => {
        if (prev.status === 'progress' && prev.step <= 1) {
          return {
            step: prev.step,
            total_steps: prev.total_steps,
            message: "启动时间较长",
            status: 'progress',  // 改为progress而不是error，因为可能还在下载
            details: "首次运行正在下载Python环境和AI模型，请继续耐心等待...",
            error: undefined
          };
        }
        return prev;
      });
    }, 300000); // 5分钟超时提示

    // Cleanup listener on unmount
    return () => {
      unlistenPromise.then(unlisten => unlisten());
      clearTimeout(startupTimeout);
    };
  }, []);

  const handleRetry = () => {
    // Reload the application
    window.location.reload();
  };

  if (isStarting) {
    return (
      <SplashScreen
        step={startupProgress.step}
        totalSteps={startupProgress.total_steps}
        message={startupProgress.message}
        status={startupProgress.status}
        details={startupProgress.details}
        error={startupProgress.error}
        onRetry={startupProgress.status === 'error' ? handleRetry : undefined}
      />
    );
  }

  return (
    <Layout
      selectedProjectId={selectedProjectId}
      onProjectSelect={setSelectedProjectId}
      theme={theme}
      onToggleTheme={toggleTheme}
    />
  );
}

export default App;
