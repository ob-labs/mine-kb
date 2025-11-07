import React from 'react';

interface SplashScreenProps {
  step: number;
  totalSteps: number;
  message: string;
  status: 'progress' | 'success' | 'error';
  details?: string;
  error?: string;
  onRetry?: () => void;
}

const SplashScreen: React.FC<SplashScreenProps> = ({
  step,
  totalSteps,
  message,
  status,
  details,
  error,
  onRetry,
}) => {
  const progress = (step / totalSteps) * 100;

  return (
    <div className="fixed inset-0 bg-gradient-to-br from-blue-50 to-indigo-100 dark:from-gray-900 dark:to-gray-800 flex items-center justify-center z-50 animate-fadeIn">
      <div className="w-full max-w-md px-8 animate-slideUp">
        {/* Logo and Title */}
        <div className="text-center mb-12">
          <div className="mb-6 flex justify-center">
            <img 
              src="/logo.gif" 
              alt="MineKB Logo" 
              className="w-8 h-8"
              style={{ width: '64px', height: '64px' }}
            />
          </div>
          <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-2">
            MineKB
          </h1>
          <p className="text-gray-600 dark:text-gray-400 text-sm">
            智能知识库管理系统
          </p>
        </div>

        {/* Progress Card */}
        <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl p-8 backdrop-blur-lg bg-opacity-90 dark:bg-opacity-90">
          {status === 'error' ? (
            // Error State
            <div className="text-center">
              <div className="mb-4">
                <svg
                  className="w-16 h-16 text-red-500 mx-auto"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </div>
              <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
                启动失败
              </h3>
              <p className="text-gray-600 dark:text-gray-400 mb-4">
                {error || '应用启动时遇到问题'}
              </p>
              {details && (
                <div className="bg-red-50 dark:bg-red-900/20 rounded-lg p-4 mb-4 text-left">
                  <p className="text-sm text-red-800 dark:text-red-300 font-mono whitespace-pre-wrap">
                    {details}
                  </p>
                </div>
              )}
              {onRetry && (
                <button
                  onClick={onRetry}
                  className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                >
                  重新启动
                </button>
              )}
            </div>
          ) : (
            // Loading State
            <>
              {/* Step Indicator */}
              <div className="flex justify-between items-center mb-6">
                {Array.from({ length: totalSteps }).map((_, index) => (
                  <React.Fragment key={index}>
                    <div
                      className={`w-7 h-7 rounded-full flex items-center justify-center text-sm font-semibold transition-all duration-300 ${
                        index < step
                          ? 'bg-green-500 text-white'
                          : index === step - 1
                          ? 'bg-blue-500 text-white ring-4 ring-blue-200 dark:ring-blue-800'
                          : 'bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400'
                      }`}
                    >
                      {index < step ? (
                        <svg
                          className="w-4 h-4"
                          fill="currentColor"
                          viewBox="0 0 20 20"
                        >
                          <path
                            fillRule="evenodd"
                            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                            clipRule="evenodd"
                          />
                        </svg>
                      ) : (
                        index + 1
                      )}
                    </div>
                    {index < totalSteps - 1 && (
                      <div
                        className={`flex-1 h-1 mx-2 rounded transition-all duration-300 ${
                          index < step - 1
                            ? 'bg-green-500'
                            : 'bg-gray-200 dark:bg-gray-700'
                        }`}
                      />
                    )}
                  </React.Fragment>
                ))}
              </div>

              {/* Progress Bar */}
              <div className="mb-6">
                <div className="flex justify-between items-center mb-2">
                  <span className="text-xs font-medium text-gray-700 dark:text-gray-300">
                    步骤 {step}/{totalSteps}
                  </span>
                  <span className="text-xs font-medium text-blue-600 dark:text-blue-400">
                    {Math.round(progress)}%
                  </span>
                </div>
                <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1 overflow-hidden">
                  <div
                    className="bg-blue-500 h-1 rounded-full transition-all duration-500 ease-out"
                    style={{ width: `${progress}%` }}
                  >
                    <div className="h-full w-full bg-white/30 animate-pulse" />
                  </div>
                </div>
              </div>

              {/* Status Message */}
              <div className="space-y-3">
                <div className="flex items-start space-x-3">
                  <div className="flex-shrink-0 mt-1">
                    {status === 'success' ? (
                      <svg
                        className="w-5 h-5 text-green-500"
                        fill="currentColor"
                        viewBox="0 0 20 20"
                      >
                        <path
                          fillRule="evenodd"
                          d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                          clipRule="evenodd"
                        />
                      </svg>
                    ) : (
                      <div className="w-5 h-5">
                        <svg
                          className="animate-spin text-blue-500"
                          fill="none"
                          viewBox="0 0 24 24"
                        >
                          <circle
                            className="opacity-25"
                            cx="12"
                            cy="12"
                            r="10"
                            stroke="currentColor"
                            strokeWidth="4"
                          />
                          <path
                            className="opacity-75"
                            fill="currentColor"
                            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                          />
                        </svg>
                      </div>
                    )}
                  </div>
                  <div className="flex-1">
                    <p className="text-xs font-medium text-gray-900 dark:text-white">
                      {message}
                    </p>
                    {details && (
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {details}
                      </p>
                    )}
                  </div>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="text-center mt-8">
          <p className="text-xs text-gray-500 dark:text-gray-500">
            MineKB v0.1.0 • Powered by SeekDB
          </p>
        </div>
      </div>
    </div>
  );
};

export default SplashScreen;

