import React from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  maxWidth?: string;
  disabled?: boolean;
}

const Modal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  title,
  children,
  footer,
  maxWidth = 'max-w-2xl',
  disabled = false,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className={`bg-card rounded-lg shadow-xl w-full ${maxWidth} max-h-[90vh] flex flex-col border border-border`}>
        {/* 头部 */}
        <div className="flex justify-between items-center p-6 border-b border-border flex-shrink-0">
          <h2 className="text-xl font-semibold text-foreground">{title}</h2>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
            disabled={disabled}
          >
            <X size={24} />
          </button>
        </div>

        {/* 内容 */}
        <div className="flex-1 overflow-y-auto min-h-0">
          {children}
        </div>

        {/* 底部 */}
        {footer && (
          <div className="border-t border-border bg-secondary flex-shrink-0">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
};

export default Modal;
