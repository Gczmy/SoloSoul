import { useUiStore } from '@/stores/uiStore';
import styles from './ToastContainer.module.css';

export function ToastContainer() {
  const { toasts, dismissToast } = useUiStore();

  if (toasts.length === 0) return null;

  return (
    <div className={styles.container}>
      {toasts.map((toast) => (
        <div key={toast.id} className={`${styles.toast} ${styles[toast.type]}`}>
          <span className={styles.message}>{toast.message}</span>
          <button className={styles.close} onClick={() => dismissToast(toast.id)}>
            x
          </button>
        </div>
      ))}
    </div>
  );
}
