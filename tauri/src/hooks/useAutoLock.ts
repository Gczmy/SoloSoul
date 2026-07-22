import { useEffect } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useVaultStore } from '@/stores/vaultStore';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';
import { sendSystemNotificationWithFallback } from '@/lib/notification';
import { addPluginListener, type PluginListener } from '@tauri-apps/api/core';
import i18next from '@/lib/i18n';

/** 视为用户活动的事件（被动监听，不干扰交互） */
const ACTIVITY_EVENTS = ['mousemove', 'mousedown', 'keydown', 'wheel', 'touchstart'] as const;
/** 闲置检查周期 */
const CHECK_INTERVAL_MS = 5_000;
/** 活动记录节流：高频事件（mousemove/wheel）最多每 1s 刷新一次时间戳 */
const ACTIVITY_THROTTLE_MS = 1_000;

/**
 * 自动锁定 —— 已认证且 `autoLockTimeoutMinutes > 0` 时生效。
 *
 * - 无用户活动超过阈值后调用 `vaultStore.lock()`，后续清理由
 *   Rust 端 `vault-locked` 事件的既有监听链（AppRoutes）完成。
 * - 移动端切后台 / 系统休眠期间定时器可能被挂起，因此在
 *   `visibilitychange` 回到前台时立即结算一次，隐藏时间计入闲置。
 * - `autoLockPauseStore` 计数 > 0（如密码验证框打开）期间暂停闲置累计。
 */
export function useAutoLock(): void {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const timeoutMinutes = useSettingsStore((s) => s.settings.autoLockTimeoutMinutes);
  const autoLockNotificationEnabled = useSettingsStore(
    (s) => s.settings.autoLockNotificationEnabled,
  );
  const autoLockOnBackground = useSettingsStore((s) => s.settings.autoLockOnBackground);

  useEffect(() => {
    if (!isAuthenticated || timeoutMinutes <= 0) return;

    const timeoutMs = timeoutMinutes * 60_000;
    let lastActivity = Date.now();
    let lastWrite = 0;
    let lockInitiated = false;

    const recordActivity = () => {
      const now = Date.now();
      if (now - lastWrite < ACTIVITY_THROTTLE_MS) return;
      lastWrite = now;
      lastActivity = now;
    };

    const checkIdle = () => {
      if (lockInitiated) return;
      // 暂停期间把闲置起点推到现在，等效于暂停计时
      if (useAutoLockPauseStore.getState().pauseCount > 0) {
        lastActivity = Date.now();
        return;
      }
      if (Date.now() - lastActivity >= timeoutMs) {
        lockInitiated = true;

        // 若用户开启「自动锁定通知」，在锁定前发送一次系统通知
        if (autoLockNotificationEnabled) {
          const body = i18next.t('settings:auto_locked_notification');
          sendSystemNotificationWithFallback('SoloSoul', body, body, 'info').catch((err) =>
            console.error('[useAutoLock] notification failed:', err),
          );
        }

        useVaultStore
          .getState()
          .lock()
          .catch((err) => console.error('[useAutoLock] lock failed:', err));
      }
    };

    const doLock = () => {
      if (lockInitiated) return;
      lockInitiated = true;
      if (autoLockNotificationEnabled) {
        const body = i18next.t('settings:auto_locked_notification');
        sendSystemNotificationWithFallback('SoloSoul', body, body, 'info').catch((err) =>
          console.error('[useAutoLock] notification failed:', err),
        );
      }
      useVaultStore
        .getState()
        .lock()
        .catch((err) => console.error('[useAutoLock] lock failed:', err));
    };

    const onVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        checkIdle();
      } else if (document.visibilityState === 'hidden') {
        // 暂停期间（如文件选择器打开）跳过锁定
        if (useAutoLockPauseStore.getState().pauseCount > 0) return;
        if (lockInitiated) return;
        // 切后台同步决策：不再 invoke('is_screen_locked')（IPC 在隐藏时不可靠），
        // 改为仅依据开关。锁屏事件由原生侧 onPause + KeyguardManager 推送触发。
        if (autoLockOnBackground) {
          doLock();
        }
      }
    };

    // 监听原生锁屏事件（Android onPause + KeyguardManager）
    // 使用插件监听器（addPluginListener）而非全局事件 listen：
    // Kotlin 侧 Plugin.trigger 只派发给插件私有 Channel 监听器，与全局事件总线无关。
    let screenLockedListener: PluginListener | null = null;
    addPluginListener<{ locked: boolean }>('lock-state', 'screen-locked', () => {
      doLock();
    })
      .then((l) => {
        screenLockedListener = l;
      })
      .catch((err) => console.error('[useAutoLock] listen screen-locked failed:', err));

    for (const e of ACTIVITY_EVENTS) {
      window.addEventListener(e, recordActivity, { passive: true });
    }
    document.addEventListener('visibilitychange', onVisibilityChange);
    const interval = setInterval(checkIdle, CHECK_INTERVAL_MS);

    return () => {
      for (const e of ACTIVITY_EVENTS) {
        window.removeEventListener(e, recordActivity);
      }
      document.removeEventListener('visibilitychange', onVisibilityChange);
      clearInterval(interval);
      screenLockedListener?.unregister()?.catch(() => {});
    };
  }, [isAuthenticated, timeoutMinutes, autoLockNotificationEnabled, autoLockOnBackground]);

}
