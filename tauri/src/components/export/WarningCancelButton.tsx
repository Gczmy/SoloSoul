import { Button } from '@/components/ui/Button';

interface WarningCancelButtonProps {
  onClick: () => void;
  children: string;
}

/** 风险确认中的取消操作沿用平台次要按钮，警告色保留给继续操作。 */
export function WarningCancelButton({ onClick, children }: WarningCancelButtonProps) {
  return (
    <Button type="button" size="sm" variant="secondary" onClick={onClick}>
      {children}
    </Button>
  );
}
