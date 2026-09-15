import { useEffect, useId, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useOverlayBackGuard } from '@/hooks/useOverlayBackGuard';

/** 仅在打开时挂载：沿用安卓返回栈守卫，Portal 避免被内容区裁剪。 */
export function AndroidSheet({
  title,
  children,
  onClose,
  innerOpen = false,
  onBack = onClose,
  trigger,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  innerOpen?: boolean;
  onBack?: () => void;
  trigger?: HTMLElement | null;
}) {
  const { t } = useTranslation('common');
  const titleId = useId();
  const panel = useRef<HTMLDivElement>(null);
  const closeRef = useRef(onBack);
  closeRef.current = innerOpen ? onBack : onClose;
  useOverlayBackGuard({ innerOpen, onCloseInner: onBack, onClose });
  useEffect(() => {
    const previous = trigger ?? document.activeElement;
    const app = document.getElementById('root');
    const wasInert = app?.inert ?? false;
    if (app) app.inert = true;
    const focusables = () =>
      Array.from(
        panel.current?.querySelectorAll<HTMLElement>(
          'button:not(:disabled), input:not(:disabled), select, textarea, [tabindex="0"]',
        ) ?? [],
      ).filter((node) => node.getClientRects().length > 0);
    (
      panel.current?.querySelector<HTMLElement>('[data-initial-focus]') ??
      focusables()[0] ??
      panel.current
    )?.focus();
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        closeRef.current();
      } else if (event.key === 'Tab') {
        const list = focusables();
        const first = list[0];
        const last = list[list.length - 1];
        if (!first) {
          event.preventDefault();
          panel.current?.focus();
        } else if (
          event.shiftKey &&
          (document.activeElement === first || !panel.current?.contains(document.activeElement))
        ) {
          event.preventDefault();
          last.focus();
        } else if (
          !event.shiftKey &&
          (document.activeElement === last || !panel.current?.contains(document.activeElement))
        ) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener('keydown', keydown, true);
    return () => {
      document.removeEventListener('keydown', keydown, true);
      if (app) app.inert = wasInert;
      if (previous instanceof HTMLElement && previous.isConnected)
        previous.focus({ preventScroll: true });
    };
  }, [trigger]);
  useEffect(() => {
    // 新建页面/返回选择层时焦点随内容移动，避免停在已卸载的输入框。
    (
      panel.current?.querySelector<HTMLElement>(innerOpen ? 'input' : '[data-initial-focus]') ??
      panel.current
    )?.focus();
  }, [innerOpen]);
  return createPortal(
    <div className="android-sheet-layer" onClick={(event) => event.stopPropagation()}>
      <div className="android-sheet-scrim" onClick={onClose} />
      <div
        ref={panel}
        className="android-sheet android-glass-surface"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
      >
        <div className="android-sheet-handle" aria-hidden="true" />
        <header className="android-sheet-heading">
          <h2 id={titleId}>{title}</h2>
          <button
            type="button"
            className="android-icon-button"
            aria-label={t('close')}
            onClick={onClose}
          >
            <X size={24} />
          </button>
        </header>
        {children}
      </div>
    </div>,
    document.body,
  );
}
