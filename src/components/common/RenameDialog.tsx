import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

interface RenameDialogProps {
  isOpen: boolean;
  currentName: string;
  onClose: () => void;
  onConfirm: (newName: string) => void;
  title?: string;
  description?: string;
  isLoading?: boolean;
}

const RenameDialog: React.FC<RenameDialogProps> = ({
  isOpen,
  currentName,
  onClose,
  onConfirm,
  title = '重命名',
  description = '请输入新名称',
  isLoading = false,
}) => {
  const [newName, setNewName] = useState(currentName);
  const [error, setError] = useState<string>('');

  useEffect(() => {
    if (isOpen) {
      setNewName(currentName);
      setError('');
    }
  }, [isOpen, currentName]);

  const handleConfirm = () => {
    if (isLoading) return;

    const trimmedName = newName?.trim();
    if (!trimmedName) {
      setError('名称不能为空');
      return;
    }
    if (trimmedName === currentName) {
      setError('新名称与原名称相同');
      return;
    }
    onConfirm(trimmedName);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirm();
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <Input
            type="text"
            value={newName}
            onChange={(e) => {
              setNewName(e?.target?.value);
              setError('');
            }}
            onKeyDown={handleKeyDown}
            placeholder="请输入名称"
            autoFocus
          />
          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}
        </div>
        <DialogFooter>
          <Button onClick={handleConfirm} disabled={isLoading}>
            {isLoading ? '处理中...' : '确定'}
          </Button>
          <Button variant="outline" onClick={onClose} disabled={isLoading}>
            取消
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default RenameDialog;

