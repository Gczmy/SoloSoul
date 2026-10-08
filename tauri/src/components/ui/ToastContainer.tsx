import { useUiStore } from '@/stores/uiStore';
import styles from './ToastContainer.module.css';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { isAndroidSync } from '@/lib/platform';
import { useToastOutletStore } from '@/stores/toastOutletStore';

export function ToastContainer() {
  const { t } = useTranslation('common');
  const target = useToastOutletStore((state) => state.target);
  const android = isAndroidSync();
  // P215: 字段级选择器——仅订阅 toasts/dismissToast，避免 uiStore 任意字段变化重渲染容器。
  const toasts = useUiStore((s) => s.toasts);
  const dismissToast = useUiStore((s) => s.dismissToast);

  if (toasts.length === 0) return null;

  const handleAction = (toast: (typeof toasts)[number]) => {
    if (toast.action) {
      toast.action.onClick();
    }
    dismissToast(toast.id);
  };

  const content = (
    <div
      className={`${styles.container} ${android ? (target ? styles.inline : styles.mobileFallback) : ''}`}
      data-toast-container
      onClick={(event) => event.stopPropagation()}
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          data-macos-glass="notification"
          className={`${styles.toast} ${styles[toast.type]} ${toast.action ? styles.clickable : ''}`}
          onClick={toast.action ? () => handleAction(toast) : undefined}
          role={toast.action ? 'button' : undefined}
          tabIndex={toast.action ? 0 : undefined}
          onKeyDown={
            toast.action
              ? (e: React.KeyboardEvent) => {
                  if (e.target !== e.currentTarget) return;
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    handleAction(toast);
                  }
                }
              : undefined
          }
        >
          <span className={styles.message}>{toast.message}</span>
          <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
            {toast.action && (
              <button
                className={styles.actionBtn}
                onClick={(e) => {
                  e.stopPropagation();
                  handleAction(toast);
                }}
              >
                {toast.action.label}
              </button>
            )}
            <button
              className={styles.close}
              aria-label={t('close')}
              onClick={(e) => {
                e.stopPropagation();
                dismissToast(toast.id);
              }}
            >
              x
            </button>
          </div>
        </div>
      ))}
    </div>
  );
  return android ? createPortal(content, target ?? document.body) : content;
}
