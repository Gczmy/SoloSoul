import { useState, useCallback } from 'react';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';

interface ConfirmState {
  isOpen: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm?: () => void;
  priority?: 'default' | 'important' | 'auth';
}

export function useConfirm() {
  const [state, setState] = useState<ConfirmState>({
    isOpen: false,
    title: '',
    message: '',
  });

  const requestConfirm = useCallback(
    (
      title: string,
      message: string,
      onConfirm: () => void,
      options?: {
        confirmLabel?: string;
        cancelLabel?: string;
        priority?: 'default' | 'important' | 'auth';
      },
    ) => {
      setState({
        isOpen: true,
        title,
        message,
        confirmLabel: options?.confirmLabel,
        cancelLabel: options?.cancelLabel,
        onConfirm,
        priority: options?.priority ?? 'default',
      });
    },
    [],
  );

  const close = useCallback(() => {
    setState((s) => ({ ...s, isOpen: false }));
  }, []);

  const dialog = (
    <ConfirmDialog
      isOpen={state.isOpen}
      title={state.title}
      message={state.message}
      confirmLabel={state.confirmLabel}
      cancelLabel={state.cancelLabel}
      onConfirm={() => {
        close();
        state.onConfirm?.();
      }}
      onCancel={close}
      priority={state.priority}
    />
  );

  return { requestConfirm, close, dialog };
}
