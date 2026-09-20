import type { ReactNode } from 'react';
import { Button } from '@/components/ui/Button';

interface TransferButtonProps {
  onClick: () => void;
  disabled?: boolean;
  busy?: boolean;
  variant?: 'plain' | 'accent' | 'warning';
  children: ReactNode;
}

/** 导入导出复用平台按钮；忙碌期间禁用，防止重复执行同一操作。 */
export function TransferButton({
  onClick,
  disabled = false,
  busy = false,
  variant = 'plain',
  children,
}: TransferButtonProps) {
  return (
    <Button
      type="button"
      size="sm"
      variant={variant === 'accent' ? 'primary' : variant === 'warning' ? 'warning' : 'secondary'}
      onClick={onClick}
      disabled={disabled}
      loading={busy}
      aria-busy={busy}
    >
      {children}
    </Button>
  );
}
