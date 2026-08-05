import { useState, useEffect, useRef } from 'react';
import { BrowserRouter } from 'react-router-dom';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { AccountInfo } from '@/lib/ipc';
import { ST_ONBOARDING_SEEN, ST_ONBOARDING_SAF_URI } from '@/lib/constants';
import { ToastContainer } from '@/components/ui/ToastContainer';
import { OcrScanNotificationListener } from '@/components/layout/OcrScanNotificationListener';
import { GlobalSyncIndicator } from '@/components/layout/GlobalSyncIndicator';
import { PluginQuickNotificationListener } from '@/components/plugin/PluginQuickNotificationListener';
import { OnboardingDialog } from '@/components/onboarding/OnboardingDialog';
import { AppRoutes } from './AppRoutes';
import { useSyncStore } from '@/stores/syncStore';
import { useUiStore } from '@/stores/uiStore';

import { initPlatform } from '@/lib/platform';
import { syncPlaintextPref } from '@/stores/settingsStore';

function App() {
  const [hasSeenOnboarding, setHasSeenOnboarding] = useState(() => {
    try {
      return localStorage.getItem(ST_ONBOARDING_SEEN) === 'true';
    } catch {
      return true;
    }
  });
  // 从「创建新账户」页点击「返回账户来源选择」时置位，重新挂载引导并直接显示账户来源卡片。
  const reopenAccountSource = useUiStore((s) => s.reopenAccountSource);

  useEffect(() => {
    initPlatform().catch(() => {
      /* ignore */
    });
  }, []);

  // 设备自动同步：应用切回前台时触发一次同步，但最多每分钟一次，避免反复切换应用导致同步风暴。
  const lastForegroundSyncRef = useRef<number>(0);
  useEffect(() => {
    const onVisibilityChange = () => {
      if (!document.hidden) {
        const now = Date.now();
        if (now - lastForegroundSyncRef.current >= 60_000) {
          lastForegroundSyncRef.current = now;
          useSyncStore.getState().triggerForegroundSync();
        }
      }
    };
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => document.removeEventListener('visibilitychange', onVisibilityChange);
  }, []);

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      invoke<{ hasSeenOnboarding?: boolean }>('ui_get_preferences').catch(() => ({
        hasSeenOnboarding: false,
      })),
      invoke<AccountInfo[]>('vault_list_accounts').catch(() => [] as AccountInfo[]),
    ])
      .then(([prefs, accounts]) => {
        if (cancelled) return;
        // If the user already has at least one account, they have clearly completed
        // onboarding before — hide the tutorial regardless of the UI pref flag.
        if (accounts.length > 0) {
          setHasSeenOnboarding(true);
          return;
        }
        const ipcSeen = prefs.hasSeenOnboarding === true;
        if (ipcSeen) {
          setHasSeenOnboarding(true);
          return;
        }
        // IPC is authoritative: if it says not seen, ignore stale localStorage
        setHasSeenOnboarding(false);
      })
      .catch(() => {
        if (cancelled) return;
        // Fallback to localStorage already applied in initial state
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const finishOnboarding = () => {
    try {
      localStorage.setItem(ST_ONBOARDING_SEEN, 'true');
      // 清除引导过程中缓存的 SAF 目录选择，避免影响后续重新安装/重置
      localStorage.removeItem(ST_ONBOARDING_SAF_URI);
    } catch {
      /* ignore */
    }
    // N-8: ③ 副本写入收敛到 syncPlaintextPref（P129 唯一写入点）；失败仅记日志，
    // localStorage 兜底已在上方 setItem。
    void syncPlaintextPref('hasSeenOnboarding', 'true');
    // 清除「返回账户来源选择」标志，避免引导结束后重新挂载
    useUiStore.getState().setReopenAccountSource(false);
    setHasSeenOnboarding(true);
  };

  return (
    // 导航同步化（useTransitions={false}）：react-router v7 的 BrowserRouter 默认把每次
    // 导航状态更新包进 React.startTransition（异步延迟）。弹层场景（搜索卡片跳转设置等）
    // 中 onClose 是紧急更新会先提交——卡片先消失露出底层页面一帧，随后 transition 才提交
    // 跳转，产生闪烁；且 flushSync 无法强制冲刷 transition 更新，无法在弹层侧规避。
    // 禁用 transition 后导航与 onClose 同事件内自动批处理为一次提交，同帧完成无闪烁。
    <BrowserRouter useTransitions={false}>
      <AppRoutes />
      <ToastContainer />
      <GlobalSyncIndicator />
      <OcrScanNotificationListener />
      <PluginQuickNotificationListener />
      {(!hasSeenOnboarding || reopenAccountSource) && (
        <OnboardingDialog
          initialShowAccountSource={reopenAccountSource}
          onComplete={finishOnboarding}
          onSkip={finishOnboarding}
        />
      )}
    </BrowserRouter>
  );
}

export default App;
