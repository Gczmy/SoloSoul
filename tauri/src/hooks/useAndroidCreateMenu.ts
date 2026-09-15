import { useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { addPluginListener, type PluginListener } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { invokeCommand } from '@/lib/ipcClient';
import { withTimeout } from '@/lib/withTimeout';
import { androidMaterialTokens } from '@/lib/androidMaterial';
import {
  requestAndroidGlassMenu,
  type AndroidCreateAction,
  type AndroidGlassCapabilities,
} from '@/lib/androidGlass';
import { useAndroidGlassMode, useMediaPreference } from './useAndroidGlass';

/** 原生菜单拥有自己的返回键；仅回退到 React 菜单后才注册网页浮层返回栈。 */
export function useAndroidCreateMenu(
  onAction: (action: AndroidCreateAction | 'unavailable') => void,
) {
  const { t } = useTranslation(['common', 'navigation']);
  const mode = useAndroidGlassMode();
  const userReduceMotion = useSettingsStore((s) => s.settings.reduceMotion);
  const systemReduceMotion = useMediaPreference('(prefers-reduced-motion: reduce)');
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const authenticated = useAuthStore((s) => s.isAuthenticated);
  const { key } = useLocation();
  const [busy, setBusy] = useState(false);
  const active = useRef<{ cancelled: boolean; cancel?: () => Promise<void> } | null>(null);
  const callback = useRef(onAction);
  callback.current = onAction;

  useEffect(() => {
    const cancel = () => {
      const request = active.current;
      active.current = null;
      if (request) {
        request.cancelled = true;
        void request.cancel?.();
      }
      setBusy(false);
    };
    const visibility = () => {
      if (document.hidden) cancel();
    };
    document.addEventListener('visibilitychange', visibility);
    return () => {
      document.removeEventListener('visibilitychange', visibility);
      cancel();
    };
  }, [key, accountId, authenticated, mode]);

  const open = async (trigger: HTMLButtonElement | null) => {
    if (active.current || !authenticated || document.hidden) return;
    if (mode !== 'enhanced') {
      callback.current('unavailable');
      return;
    }
    const token: { cancelled: boolean; cancel?: () => Promise<void> } = { cancelled: false };
    active.current = token;
    setBusy(true);
    let listener: PluginListener | undefined;
    const current = () =>
      !token.cancelled &&
      active.current === token &&
      useAuthStore.getState().isAuthenticated &&
      useAuthStore.getState().currentAccount?.id === accountId &&
      !document.hidden;
    try {
      const capabilities = await withTimeout(
        invokeCommand<AndroidGlassCapabilities>('android_glass_capabilities'),
        1000,
      );
      if (!current()) return;
      if (!capabilities?.windowBlur) {
        callback.current('unavailable');
        return;
      }
      // 能力变化只更新诊断标记；已打开窗口由 Kotlin 原位提高底色，不突然换成另一个菜单。
      const registration = addPluginListener<AndroidGlassCapabilities>(
        'android-glass',
        'capabilities-changed',
        (caps) => {
          if (current())
            document.documentElement.dataset.androidWindowBlur = String(caps.windowBlur);
        },
      );
      // 挂载中途取消也必须注销迟到的监听器。
      registration
        .then((value) => {
          if (!current()) void value.unregister().catch(() => {});
        })
        .catch(() => {});
      listener = await withTimeout(registration, 1000);
      if (!current()) return;
      const dark = document.documentElement.dataset.theme === 'dark';
      const colors = androidMaterialTokens(dark, useSettingsStore.getState().settings.accentColor);
      const operation = requestAndroidGlassMenu({
        requestId: crypto.randomUUID(),
        title: t('material.new_title'),
        description: t('material.create_description'),
        closeLabel: t('close'),
        footer: t('material.local_vault'),
        labels: {
          object: t('material.new_object'),
          page: t('navigation:add_page'),
          scan: t('material.scan'),
        },
        descriptions: {
          object: t('material.new_object_desc'),
          page: t('material.new_page_desc'),
          scan: t('material.scan_desc'),
        },
        dark,
        reduceMotion: userReduceMotion || systemReduceMotion,
        background: colors['--bg-base'],
        foreground: colors['--text-primary'],
        secondary: colors['--text-secondary'],
        accent: colors['--accent-primary'],
        container: colors['--md-primary-container'],
      });
      token.cancel = operation.cancel;
      const action = await operation.result;
      if (current() && action && action !== 'cancel') callback.current(action);
    } catch {
      // 原生路径异常不能让 FAB 失效；关闭可能已出现的窗口，再回退至已有 React 菜单。
      if (token.cancel) await withTimeout(token.cancel(), 1000).catch(() => {});
      if (current()) callback.current('unavailable');
    } finally {
      void listener?.unregister().catch(() => {});
      if (active.current === token) {
        active.current = null;
        setBusy(false);
        trigger?.focus({ preventScroll: true });
      }
    }
  };
  return { open, busy };
}
