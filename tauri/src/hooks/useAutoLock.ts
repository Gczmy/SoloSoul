import { useEffect } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';
import { sendSystemNotificationWithFallback } from '@/lib/notification';
import { addPluginListener, type PluginListener } from '@tauri-apps/api/core';
import { invoke } from '@tauri-apps/api/core';
import i18next from '@/lib/i18n';
import { logger } from '@/lib/logger';

/** 视为用户活动的事件（被动监听，不干扰交互） */
const ACTIVITY_EVENTS = ['mousemove', 'mousedown', 'keydown', 'wheel', 'touchstart'] as const;
/** 闲置检查周期 */
const CHECK_INTERVAL_MS = 5_000;
/** 活动记录节流：高频事件（mousemove/wheel）最多每 1s 刷新一次时间戳 */
const ACTIVITY_THROTTLE_MS = 1_000;

/**
 * 自动锁定 —— 已认证且 `autoLockTimeoutMinutes > 0` 时生效。
 *
 * - 无用户活动超过阈值后调用 `authStore.lock()`，乐观重置认证状态；
 *   后续清理由 Rust 端 `vault-locked` 事件的既有监听链（AppRoutes）兜底完成。
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
            logger.error('[useAutoLock] notification failed:', err),
          );
        }

        useAuthStore
          .getState()
          .lock()
          .catch((err) => logger.error('[useAutoLock] lock failed:', err));
      }
    };

    const triggerBackgroundSync = () => {
      // 切后台时触发一次 SAF 后台同步（仅在 Android SAF 模式下有效）。
      // 使用 fire-and-forget，不等待结果，避免 WebView 冻结时挂起。
      invoke('vault_sync_background').catch((err) =>
        logger.warn('[useAutoLock] vault_sync_background failed:', err),
      );
    };

    const doLock = () => {
      if (lockInitiated) return;
      lockInitiated = true;
      if (autoLockNotificationEnabled) {
        const body = i18next.t('settings:auto_locked_notification');
        sendSystemNotificationWithFallback('SoloSoul', body, body, 'info').catch((err) =>
          logger.error('[useAutoLock] notification failed:', err),
        );
      }
      useAuthStore
        .getState()
        .lock()
        .catch((err) => logger.error('[useAutoLock] lock failed:', err));
      // 主动撤掉原生锁屏遮盖层：当 autoLockOnBackground 开启时，
      // doLock 先于 Kotlin 的延迟检测触发锁定，清除 isAuthenticated
      // 导致第二个 useEffect 被清理，后续原生 markLockedAndTrigger
      // 挂上的 mask 将无人处理。此处先发制人清除。
      invoke('dismiss_lock_mask').catch((err) =>
        logger.warn('[useAutoLock] dismiss_lock_mask failed:', err),
      );
    };

    const pullLockPending = () => {
      invoke<boolean>('get_lock_pending')
        .then((pending) => {
          if (pending) {
            if (lockInitiated) return;
            lockInitiated = true;
            useAuthStore.getState().lock().catch((err) =>
              logger.error('[useAutoLock] lock failed:', err),
            );
            invoke('dismiss_lock_mask').catch((err) =>
              logger.warn('[useAutoLock] dismiss_lock_mask failed:', err),
            );
          }
        })
        .catch(() => {});
    };

    const onVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        checkIdle();
        // 兜底：第二个 useEffect 的 pullLockPending 依赖 [isAuthenticated]，
        // 但 lock() 导致 isAuthenticated 变化后该 effect 会被清理。
        // 此处不依赖 isAuthenticated，始终在回前台时检查原生锁屏标记。
        pullLockPending();
      } else if (document.visibilityState === 'hidden') {
        // 暂停期间（如文件选择器打开）跳过锁定
        if (useAutoLockPauseStore.getState().pauseCount > 0) return;
        if (lockInitiated) return;
        // 切后台同步决策：不再 invoke('is_screen_locked')（IPC 在隐藏时不可靠），
        // 改为仅依据开关。锁屏事件由原生侧 onPause + KeyguardManager 推送触发。
        if (autoLockOnBackground) {
          doLock();
        }
        // 无论是否切后台锁定，都尝试触发 SAF 后台同步。
        triggerBackgroundSync();
      }
    };

    // 监听原生锁屏事件（Android onPause + KeyguardManager）
    // 使用插件监听器（addPluginListener）而非全局事件 listen：
    // Kotlin 侧 Plugin.trigger 只派发给插件私有 Channel 监听器，与全局事件总线无关。
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
    };
  }, [isAuthenticated, timeoutMinutes, autoLockNotificationEnabled, autoLockOnBackground]);

  // 锁屏锁定（Android）：与闲置超时设置无关，已认证即生效。
  // 原生侧标记只在 JS 确认（dismiss_lock_mask）后清除：
  // 事件因 WebView 冻结丢失时会在下次 resume 补达；
  // 每次回到前台的主动拉取则闭合「事件已丢但标记仍在」的所有环路
  // （渲染进程被回收、resume 补达早于 WebView 恢复等）。
  useEffect(() => {
    if (!isAuthenticated) return;

    const handleScreenLocked = () => {
      useAuthStore
        .getState()
        .lock()
        .catch((err) => logger.error('[useAutoLock] lock failed:', err));
      // 撤掉原生锁屏遮盖层并清除原生挂起标记（Android；其他平台为 no-op）。
      // 无论 lock 是否因已锁定而跳过都要执行，避免遮盖/标记残留。
      invoke('dismiss_lock_mask').catch((err) =>
        logger.warn('[useAutoLock] dismiss_lock_mask failed:', err),
      );
      // 锁屏时触发一次 SAF 后台同步。
      invoke('vault_sync_background').catch((err) =>
        logger.warn('[useAutoLock] vault_sync_background failed:', err),
      );
    };

    // 回前台时主动拉取未确认的锁屏挂起标记：
    // resume 补达可能早于 WebView 恢复 JS 处理而丢失，拉取是确定性兜底。
    const pullLockPending = () => {
      invoke<boolean>('get_lock_pending')
        .then((pending) => {
          if (pending) handleScreenLocked();
        })
        .catch(() => {});
    };
    const onForeground = () => {
      if (document.visibilityState === 'visible') pullLockPending();
    };

    let screenLockedListener: PluginListener | null = null;
    addPluginListener<{ locked: boolean }>('lock-state', 'screen-locked', handleScreenLocked)
      .then((l) => {
        screenLockedListener = l;
      })
      .catch((err) => logger.error('[useAutoLock] listen screen-locked failed:', err));

    // 启动/认证后立即拉取一次，并在此后每次回到前台时拉取
    pullLockPending();
    document.addEventListener('visibilitychange', onForeground);

    return () => {
      screenLockedListener?.unregister()?.catch(() => {});
      document.removeEventListener('visibilitychange', onForeground);
    };
  }, [isAuthenticated]);
}
