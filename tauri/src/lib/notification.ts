import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { useUiStore } from '@/stores/uiStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { syncPlaintextPref } from '@/stores/settingsStore';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import i18next from '@/lib/i18n';
import { navigateTo } from '@/lib/navigation';
import { logger } from '@/lib/logger';

/**
 * 申请系统通知权限（应用级最多弹一次系统对话框）。
 *
 * - 已授权（任一账户授权过，或用户在系统设置手动开启）：直接返回 true；
 * - 未授权且本机从未请求过：弹系统对话框，并把"已请求"标记写入
 *   ui_preferences.json（应用级、设备本地，不随账户/Vault 同步）；
 * - 未授权但已请求过：不再弹窗（与 Android 13+ 行为一致），返回 false，
 *   由调用方走应用内 toast 兜底。
 *
 * 系统权限弹窗会触发 visibilitychange，期间暂停自动锁定，
 * 避免用户点「允许/拒绝」后回到应用发现已被锁定。
 */
async function requestNotificationPermissionOnce(): Promise<boolean> {
  if (await isPermissionGranted()) return true;

  let alreadyRequested = false;
  try {
    const prefs = await invoke<{ notificationPermissionRequested?: boolean }>(
      'ui_get_preferences',
    );
    alreadyRequested = prefs.notificationPermissionRequested === true;
  } catch (err) {
    // 读取失败时降级为允许请求，不因存储故障阻断功能
    logger.warn('[notification] read permission-requested flag failed:', err);
  }
  if (alreadyRequested) return false;

  const { pause, resume } = useAutoLockPauseStore.getState();
  pause();
  try {
    return (await requestPermission()) === 'granted';
  } finally {
    resume();
    // 无论允许/拒绝都记录"已请求"（二次请求在 Android 13+ 本就不再弹 UI）
    // N-8: ③ 副本写入收敛到 syncPlaintextPref（P129 唯一写入点），内部已记日志。
    void syncPlaintextPref('notificationPermissionRequested', 'true');
  }
}

interface LlmStreamPayload {
  conversationId: string;
  chunk: string;
  isDone: boolean;
  error?: string;
}

/**
 * Tracks conversation IDs for which the user sent a message and is awaiting
 * an AI response. When the stream completes and the user is no longer viewing
 * the AI chat (full page or quick card), a system notification + in-app toast
 * are shown.
 */
const pendingConversations = new Set<string>();
let unlisten: UnlistenFn | null = null;

// F029: avoid querying the global DOM to determine the current page; callers
// update these flags instead.
let isAiPageOpen = false;
let isQuickChatOpen = false;

export function setAiPageOpen(open: boolean): void {
  isAiPageOpen = open;
}

export function setQuickChatOpen(open: boolean): void {
  isQuickChatOpen = open;
}

/**
 * Initialize the global LLM stream listener for notification purposes.
 * Should be called once at app startup.
 *
 * 注意：不在启动时申请通知权限，权限延迟到首次真正发送通知时由
 * sendSystemNotificationWithFallback 按需申请，避免启动即弹窗。
 */
export async function initLlmNotificationListener(): Promise<void> {
  if (unlisten) return;

  unlisten = await listen<LlmStreamPayload>('llm-stream-chunk', (event) => {
    const payload = event.payload;

    // Error or not done → don't notify yet (but still clear error ones)
    if (payload.error) {
      pendingConversations.delete(payload.conversationId);
      return;
    }
    if (!payload.isDone) return;

    if (!pendingConversations.has(payload.conversationId)) return;
    pendingConversations.delete(payload.conversationId);

    if (!isAiPageOpen && !isQuickChatOpen) {
      // 首次触发时按需申请权限，避免启动即弹窗
      sendSystemNotificationWithFallback(
        i18next.t('common:ai_notification_title', 'SoloSoul AI'),
        i18next.t('common:ai_notification_body', 'Click to view the AI response'),
        i18next.t('common:ai_notification_toast', 'AI response ready'),
        'info',
        true,
      );
    }
  });
}

/**
 * Mark a conversation as pending notification. Call this right before
 * invoking `llm_send_message_stream`.
 */
export function markConversationPending(convId: string): void {
  pendingConversations.add(convId);
}

/**
 * 发送系统通知，并在权限被拒绝时回退到应用内 toast。
 * 首次调用时会尝试申请通知权限（按需）。
 */
export async function sendSystemNotificationWithFallback(
  title: string,
  body: string,
  toastMessage?: string,
  toastType: 'info' | 'warning' | 'error' | 'success' = 'info',
  showToastAlways = false,
): Promise<void> {
  try {
    const hasPermission = await requestNotificationPermissionOnce();

    if (hasPermission) {
      sendNotification({ title, body });
    }

    if (!hasPermission || showToastAlways) {
      useUiStore.getState().showToast({
        message: toastMessage || body,
        type: toastType,
        duration: 5000,
      });
    }
  } catch (err) {
    logger.error('[notification] sendSystemNotificationWithFallback failed:', err);
    // 兜底：至少显示应用内 toast
    useUiStore.getState().showToast({
      message: toastMessage || body,
      type: toastType,
      duration: 5000,
    });
  }
}

interface BackupInfo {
  created_at: string;
}

/**
 * 检查备份提醒。若用户未备份或距上次备份超过 `backupReminderDays` 天，
 * 则发送系统通知 + 应用内 toast 引导用户前往备份页。
 * 在 Vault 解锁后延迟调用，避免启动时权限弹窗干扰。
 *
 * P228: accountId 由调用方注入（authStore/LoginPage 解锁流程），
 * 不再静态依赖 useAuthStore——断开 notification ↔ authStore 循环依赖。
 */
export async function checkBackupReminder(accountId: string | undefined): Promise<void> {
  try {
    if (!accountId) return;

    const store = useSettingsStore.getState();

    // 直接读后端权威值，规避与 loadSettings 的竞态
    // （解锁后 2s 时内存 settings 可能还是默认值）
    const prefs = await invoke<Record<string, unknown>>('user_data_get_preferences', { accountId: accountId });
    const days =
      typeof prefs.backupReminderDays === 'number'
        ? prefs.backupReminderDays
        : store.settings.backupReminderDays;
    if (days <= 0) return;

    const lastBackupReminderAt =
      typeof prefs.lastBackupReminderAt === 'number' ? prefs.lastBackupReminderAt : null;

    // 方案 A: 记录最后提醒时间，间隔 = backupReminderDays 天
    // 如果上次提醒距今不足 days 天，则跳过本次提醒
    const reminderIntervalMs = days * 24 * 60 * 60 * 1000;
    if (lastBackupReminderAt !== null && Date.now() - lastBackupReminderAt < reminderIntervalMs) {
      return;
    }

    const backups = await invoke<BackupInfo[]>('backup_list');
    let needsBackup = backups.length === 0;

    if (!needsBackup) {
      backups.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
      const lastBackupTime = new Date(backups[0].created_at).getTime();
      const diffDays = (Date.now() - lastBackupTime) / (1000 * 60 * 60 * 24);
      needsBackup = diffDays >= days;
    }

    if (needsBackup) {
      const title = i18next.t('settings:backup_reminder_title', 'SoloSoul');
      const body = i18next.t(
        'settings:backup_reminder_body',
        'It has been a while since your last backup. Please go to Settings > Backup & Restore to create one.',
      );

      // 发送系统通知（不包含 fallback toast，因为下方已有可点击 toast）
      const hasPermission = await requestNotificationPermissionOnce();
      if (hasPermission) {
        sendNotification({ title, body });
      }

      // 应用内可点击 toast，带「去备份」按钮
      useUiStore.getState().showToast({
        message: body,
        type: 'warning',
        duration: 8000,
        action: {
          label: i18next.t('settings:backup_now', '去备份'),
          onClick: () => {
            navigateTo('/settings/backup');
          },
        },
      });

      // 记录本次提醒时间并持久化到后端，避免下次解锁重复提醒
      const now = Date.now();
      useSettingsStore.setState((s) => ({
        settings: { ...s.settings, lastBackupReminderAt: now },
      }));
      // 异步持久化到后端，重启应用后仍可记忆最后提醒时间
      if (accountId) {
        useSettingsStore
          .getState()
          .updateSetting(accountId, 'lastBackupReminderAt', now)
          .catch((err) => logger.warn('[notification] Failed to persist backup reminder time:', err));
      }
    }
  } catch (err) {
    logger.warn('[notification] Backup reminder check failed:', err);
  }
}
