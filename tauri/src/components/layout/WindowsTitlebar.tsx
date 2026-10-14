import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { observeTitlebarControls } from '@/lib/nativeTitlebarControls';
import { useLayoutEffect, useRef } from 'react';
import styles from './WindowsTitlebar.module.css';

export function WindowBrand({ showName = true }: { showName?: boolean }) {
  return showName ? <span>SoloSoul</span> : null;
}

/** 认证页沿用同一原生窗框，品牌行不接管系统窗口按钮。 */
export function WindowsTitlebar() {
  const enabled = useNativeWindowStore((s) => Boolean(s.windowsFrame));
  const headerHeight = useNativeWindowStore((s) => s.windowsFrame?.headerHeight);
  const ref = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    if (!enabled || !ref.current) return;
    return observeTitlebarControls(ref.current);
  }, [enabled]);
  if (!enabled) return null;
  return (
    <header
      ref={ref}
      data-windows-titlebar
      data-tauri-drag-region={headerHeight === 0 ? 'false' : 'deep'}
      className={styles.titlebar}
    >
      <div className={styles.brand} onDragStart={(event) => event.preventDefault()}>
        <WindowBrand />
      </div>
    </header>
  );
}
