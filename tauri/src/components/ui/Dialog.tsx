import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import styles from './Dialog.module.css';

interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
  title?: string;
  /** 可选的内联样式，用于覆盖 .dialog 的默认样式（如 width、maxWidth） */
  dialogStyle?: React.CSSProperties;
  /** 层级优先级：普通弹窗用默认，重要弹窗用 'important'，密码/认证弹窗用 'auth' */
  priority?: 'default' | 'important' | 'auth';
}

export function Dialog({
  isOpen,
  onClose,
  children,
  title,
  dialogStyle,
  priority = 'default',
}: DialogProps) {
  useEffect(() => {
    if (!isOpen) return;
    // 锁定背景滚动
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    // Escape 键关闭
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = prevOverflow;
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return createPortal(
    // stopPropagation：对话框经 createPortal 渲染到 body，但仍是宿主组件的 React 子节点——
    // React 合成事件会沿组件树冒泡，若不拦截，点击输入框等会泄漏到宿主背景的 onClick
    // （如 AttachmentViewer/AttachmentPreviewOverlay 点击背景即关闭），导致对话框与宿主一起关闭。
    <div
      className={styles.wrapper}
      data-priority={priority}
      onClick={(e) => e.stopPropagation()}
    >
      {/* Backdrop overlay */}
      <div className={styles.backdrop} onClick={onClose} />
      {/* Dialog content */}
      <div
        className={styles.dialog}
        role="dialog"
        aria-modal="true"
        style={dialogStyle}
        onClick={(e) => {
          if (e.target === e.currentTarget) {
            onClose();
          }
        }}
      >
        {title && <h2 className={styles.title}>{title}</h2>}
        {children}
      </div>
    </div>,
    document.body,
  );
}
