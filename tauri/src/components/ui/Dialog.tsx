import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import styles from './Dialog.module.css';
import { ToastOutlet } from './ToastOutlet';
import { useNativeBackdropClose } from '@/hooks/useNativeBackdropClose';

interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode | ((close: () => void) => React.ReactNode);
  title?: string;
  /** 可选的内联样式，用于覆盖 .dialog 的默认样式（如 width、maxWidth） */
  dialogStyle?: React.CSSProperties;
  /** 层级优先级：普通弹窗用默认，重要弹窗用 'important'，密码/认证弹窗用 'auth' */
  priority?: 'default' | 'important' | 'auth';
  /** 可选的自定义 z-index（内联覆盖优先级层级），用于叠加在更高层覆盖物之上 */
  zIndex?: number;
}

export function Dialog({
  isOpen,
  onClose,
  children,
  title,
  dialogStyle,
  priority = 'default',
  zIndex,
}: DialogProps) {
  const { native, backdropRef, close } = useNativeBackdropClose(onClose, isOpen);
  useEffect(() => {
    if (!isOpen) return;
    // 锁定背景滚动
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    // Escape 键关闭
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = prevOverflow;
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [isOpen, close]);

  if (!isOpen) return null;

  return createPortal(
    // stopPropagation：对话框经 createPortal 渲染到 body，但仍是宿主组件的 React 子节点——
    // React 合成事件会沿组件树冒泡，若不拦截，点击输入框等会泄漏到宿主背景的 onClick
    // （如 AttachmentViewer/AttachmentPreviewOverlay 点击背景即关闭），导致对话框与宿主一起关闭。
    <div
      className={styles.wrapper}
      data-priority={priority}
      style={zIndex !== undefined ? { zIndex } : undefined}
      onClick={(e) => e.stopPropagation()}
    >
      {/* Backdrop overlay */}
      <div
        ref={backdropRef}
        className={styles.backdrop}
        data-macos-glass-backdrop
        data-native-window-backdrop={native || undefined}
        onClick={close}
      />
      {/* Dialog content */}
      <div
        className={styles.dialog}
        data-macos-glass="panel"
        data-native-backdrop-panel
        data-native-window-backdrop-content={native || undefined}
        role="dialog"
        aria-modal="true"
        style={dialogStyle}
        onClick={(e) => {
          if (e.target === e.currentTarget) {
            close();
          }
        }}
      >
        {title && <h2 className={styles.title}>{title}</h2>}
        <ToastOutlet
          priority={zIndex ?? (priority === 'auth' ? 8000 : priority === 'important' ? 5000 : 4000)}
        />
        {typeof children === 'function' ? children(close) : children}
      </div>
    </div>,
    document.body,
  );
}
