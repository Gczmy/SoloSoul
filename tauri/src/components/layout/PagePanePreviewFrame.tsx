import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from 'react';
import { createPortal } from 'react-dom';
import { shellInset } from '@/components/workspace/objectRuler';
import styles from '@/components/workspace/WorkspaceObjectRuler.module.css';

export function PagePanePreviewFrame({
  anchor,
  cardRef,
  id,
  label,
  onClose,
  onDismiss,
  onMouseEnter,
  onMouseLeave,
  children,
}: {
  anchor: RefObject<HTMLButtonElement | null>;
  cardRef: RefObject<HTMLDivElement | null>;
  id: string;
  label: string;
  onClose: () => void;
  onDismiss: () => void;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
  children: ReactNode;
}) {
  const [position, setPosition] = useState<{ left: number; top: number } | null>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useLayoutEffect(() => {
    const update = () => {
      const button = anchor.current;
      const card = cardRef.current;
      if (!button || !card) return;
      const rect = button.getBoundingClientRect();
      const list = button.closest('[data-page-pane-scroll]')?.getBoundingClientRect();
      if (list && (rect.bottom <= list.top || rect.top >= list.bottom)) {
        closeRef.current();
        return;
      }
      const availableLeft = rect.right + 8;
      const left = Math.max(8, Math.min(availableLeft, window.innerWidth - card.offsetWidth - 8));
      const top = Math.max(
        shellInset('top') + 8,
        Math.min(rect.top, window.innerHeight - shellInset('bottom') - card.offsetHeight - 8),
      );
      setPosition((previous) =>
        previous?.left === left && previous.top === top ? previous : { left, top },
      );
    };
    update();
    const observer = new ResizeObserver(update);
    if (anchor.current) observer.observe(anchor.current);
    if (cardRef.current) observer.observe(cardRef.current);
    const scroll = (event: Event) => {
      if (!(event.target instanceof Node && cardRef.current?.contains(event.target))) {
        if (document.activeElement === anchor.current) update();
        else closeRef.current();
      }
    };
    window.addEventListener('scroll', scroll, true);
    window.addEventListener('resize', update);
    return () => {
      observer.disconnect();
      window.removeEventListener('scroll', scroll, true);
      window.removeEventListener('resize', update);
    };
  }, [anchor, cardRef]);
  useEffect(() => {
    const dismiss = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onDismiss();
    };
    window.addEventListener('keydown', dismiss);
    return () => window.removeEventListener('keydown', dismiss);
  }, [onDismiss]);

  return createPortal(
    <div
      ref={cardRef}
      id={id}
      role="region"
      aria-label={label}
      data-page-pane-preview
      data-macos-glass="panel"
      className={styles.preview}
      style={{
        ...position,
        right: 'auto',
        maxWidth: 'calc(100vw - 16px)',
        visibility: position ? undefined : 'hidden',
        zIndex: 200,
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onBlur={(event) => {
        const target = event.relatedTarget;
        if (
          !(
            target instanceof Node &&
            (cardRef.current?.contains(target) || anchor.current?.contains(target))
          )
        )
          onClose();
      }}
    >
      {children}
    </div>,
    document.body,
  );
}
