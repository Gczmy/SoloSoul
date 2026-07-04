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
  const dialogRef = useRef<HTMLDialogElement>(null);

  if (!isOpen) return null;

  useEffect(() => {
    const el = dialogRef.current;
    if (!el) return;
    if (isOpen && !el.open) {
      el.showModal();
    } else if (!isOpen && el.open) {
      el.close();
    }
  }, [isOpen]);

  useEffect(() => {
    const el = dialogRef.current;
    if (!el) return;
    const handler = () => onClose();
    el.addEventListener('close', handler);
    return () => el.removeEventListener('close', handler);
  }, [onClose]);

  return (
    <dialog
      ref={dialogRef}
      className={styles.dialog}
      style={dialogStyle}
      onClick={(e) => {
        if (e.target === dialogRef.current) {
          onClose();
        }
      }}
    >
      {title && <h2 className={styles.title}>{title}</h2>}
      {children}
    </dialog>
  );
}
