import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { MoreHorizontal } from 'lucide-react';
import styles from './ToolbarActions.module.css';

/** 主操作常驻；次要操作按顶栏实际可用宽度收纳，组件保持挂载。 */
export function ToolbarActions({
  primary,
  children,
}: {
  primary?: ReactNode;
  children?: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  const secondary = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(false);
  const [open, setOpen] = useState(false);
  const { t } = useTranslation('common');
  const location = useLocation();

  useLayoutEffect(() => {
    const header = ref.current?.closest('header');
    if (!header) return;
    const measure = () => {
      const style = getComputedStyle(header);
      const available =
        header.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
      setCompact(available < 640);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(header);
    measure();
    return () => observer.disconnect();
  }, []);

  useEffect(() => setOpen(false), [location.key, compact]);
  useEffect(() => {
    if (!open) return;
    // 等待 inert 移除及可见性样式落地，再把焦点交给第一个操作。
    const frame = requestAnimationFrame(() => {
      secondary.current?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    });
    const outside = (event: PointerEvent) => {
      // 指南/对话框 Portal 已离开操作组；交互由其自身管理。
      if ((event.target as HTMLElement).closest('[data-page-guide-overlay]')) return;
      if (!ref.current?.contains(event.target as Node)) {
        if (secondary.current?.contains(document.activeElement)) toggle.current?.focus();
        setOpen(false);
      }
    };
    document.addEventListener('pointerdown', outside);
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener('pointerdown', outside);
    };
  }, [open]);

  return (
    <div ref={ref} className={styles.toolbar} data-compact={compact}>
      {primary && <div className={styles.primary}>{primary}</div>}
      {children && (
        <>
          {compact && (
            <button
              ref={toggle}
              type="button"
              className={styles.toggle}
              aria-label={t('more_actions')}
              aria-expanded={open}
              aria-controls="toolbar-secondary-actions"
              onClick={() => setOpen((value) => !value)}
            >
              <MoreHorizontal size={20} />
            </button>
          )}
          <div
            ref={secondary}
            id="toolbar-secondary-actions"
            role="group"
            aria-label={t('more_actions')}
            className={styles.secondary}
            data-open={open}
            inert={compact && !open}
            onKeyDown={(event) => {
              if (
                event.key === 'Escape' &&
                compact &&
                !event.defaultPrevented &&
                event.currentTarget.contains(event.target as Node)
              ) {
                event.stopPropagation();
                toggle.current?.focus();
                setOpen(false);
              }
            }}
          >
            {children}
          </div>
        </>
      )}
    </div>
  );
}
