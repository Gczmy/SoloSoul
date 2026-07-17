import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { useUiStore } from '@/stores/uiStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { invoke } from '@tauri-apps/api/core';
import i18next from '@/lib/i18n';

const AI_NOTIFICATION_TOAST_DURATION_MS = 5000;

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
        'SoloSoul AI',
        'AI 已完成回复，点击查看',
        'AI 已完成回复',
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
    let hasPermission = await isPermissionGranted();
    if (!hasPermission) {
      hasPermission = (await requestPermission()) === 'granted';
    }

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
    console.error('[notification] sendSystemNotificationWithFallback failed:', err);
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
 */
export async function checkBackupReminder(): Promise<void> {
  try {
    const days = useSettingsStore.getState().settings.backupReminderDays;
    if (days <= 0) return;

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
      await sendSystemNotificationWithFallback(title, body, body, 'warning', true);
    }
  } catch (err) {
    console.error('[notification] Backup reminder check failed:', err);
  }
}
