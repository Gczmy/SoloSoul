import { useEffect, useRef } from 'react';
import styles from './Dialog.module.css';

interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
  title?: string;
  /** 可选的内联样式，用于覆盖 .dialog 的默认样式（如 width、maxWidth） */
  dialogStyle?: React.CSSProperties;
}

export function Dialog({ isOpen, onClose, children, title, dialogStyle }: DialogProps) {
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      ref={overlayRef}
      className={styles.overlay}
      onClick={(e) => {
        if (e.target === overlayRef.current) {
          e.stopPropagation();
          onClose();
        }
      }}
    >
      <div className={styles.dialog} style={dialogStyle} onClick={(e) => e.stopPropagation()}>
        {title && <h2 className={styles.title}>{title}</h2>}
        {children}
      </div>
    </div>
  );
}
