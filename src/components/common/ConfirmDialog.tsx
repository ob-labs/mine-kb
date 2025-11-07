import React from 'react';
import { AlertTriangle } from 'lucide-react';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';

interface ConfirmDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  type?: 'danger' | 'warning' | 'info';
  variant?: 'danger' | 'warning' | 'info'; // 兼容旧的 API
  isLoading?: boolean;
}

const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  confirmText = '确认',
  cancelText = '取消',
  type = 'danger',
  variant, // 兼容旧的 API
  isLoading = false,
}) => {
  // 兼容旧的 variant prop
  const currentType = variant || type;

  const typeStyles = {
    danger: {
      icon: 'text-destructive',
      buttonVariant: 'destructive' as const,
    },
    warning: {
      icon: 'text-yellow-500',
      buttonVariant: 'default' as const,
    },
    info: {
      icon: 'text-primary',
      buttonVariant: 'default' as const,
    },
  };

  const currentStyle = typeStyles[currentType];

  const handleConfirm = () => {
    if (!isLoading) {
      onConfirm();
    }
  };

  const handleCancel = () => {
    if (!isLoading) {
      onClose();
    }
  };

  return (
    <AlertDialog open={isOpen} onOpenChange={onClose}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <div className="flex items-start gap-4">
            <div className={`flex-shrink-0 ${currentStyle.icon}`}>
              <AlertTriangle size={24} />
            </div>
            <div className="flex-1 min-w-0">
              <AlertDialogTitle className="text-left">{title}</AlertDialogTitle>
              <AlertDialogDescription className="text-left mt-2">
                {message}
              </AlertDialogDescription>
            </div>
          </div>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogAction asChild>
            <Button
              variant={currentStyle.buttonVariant}
              onClick={handleConfirm}
              disabled={isLoading}
            >
              {isLoading ? '处理中...' : confirmText}
            </Button>
          </AlertDialogAction>
          <AlertDialogCancel onClick={handleCancel} disabled={isLoading}>
            {cancelText}
          </AlertDialogCancel>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};

export default ConfirmDialog;

