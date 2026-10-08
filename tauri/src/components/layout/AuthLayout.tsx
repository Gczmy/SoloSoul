import { useEffect, useRef } from 'react';
import { Outlet } from 'react-router-dom';
import { isAndroidSync } from '@/lib/platform';
import { ShellNotificationSlot } from './ShellNotifications';
import styles from './AuthLayout.module.css';
import { useAndroidGlassSurface } from '@/hooks/useAndroidGlass';

/** 认证页共用布局；只分配通知与表单空间，鉴权仍由路由负责。 */
export function AuthLayout() {
  useAndroidGlassSurface();
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isAndroidSync() || !window.visualViewport) return;
    const root = rootRef.current;
    const viewport = window.visualViewport;
    let frame = 0;
    const update = () => {
      // 部分 WebView 的输入法只缩小视觉视口，100dvh 仍可能覆盖到键盘后方。
      const height = Math.min(window.innerHeight, viewport.height);
      root?.style.setProperty('--auth-viewport-height', `${height}px`);
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        // 键盘只自动保证输入框可见；同时带入该解锁方式的提交操作，避免按钮停在键盘下。
        const active = document.activeElement;
        if (active instanceof HTMLElement && root?.contains(active)) {
          active
            .closest<HTMLElement>('[data-login-method-region]')
            ?.scrollIntoView({ block: 'nearest' });
        }
      });
    };
    update();
    viewport.addEventListener('resize', update);
    viewport.addEventListener('scroll', update);
    root?.addEventListener('focusin', update);
    return () => {
      cancelAnimationFrame(frame);
      viewport.removeEventListener('resize', update);
      viewport.removeEventListener('scroll', update);
      root?.removeEventListener('focusin', update);
      root?.style.removeProperty('--auth-viewport-height');
    };
  }, []);

  return (
    <div ref={rootRef} className={styles.layout} data-auth-layout>
      <ShellNotificationSlot />
      <div className={styles.content} data-auth-content>
        <Outlet />
      </div>
    </div>
  );
}
