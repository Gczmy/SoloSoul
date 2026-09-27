import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import { checkBackupReminder } from './notification';
import { logger } from './logger';
vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn(async () => true),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
}));
vi.mock('./logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));
beforeEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'A' });
  useSettingsStore.setState((s) => ({
    settings: { ...s.settings, backupReminderDays: 7, lastBackupReminderAt: 100 },
  }));
  useUiStore.setState({ toasts: [] });
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === 'user_data_get_preferences')
      return { backupReminderDays: 7, lastBackupReminderAt: 100 };
    if (command === 'backup_list') return [];
    if (command === 'user_data_update_preference')
      throw new Error('Synthetic preference write failure');
    return undefined;
  });
});
it('RF111 备份提醒已经发出，但时间保存失败时恢复旧时间且不误报通知发送失败', async () => {
  await checkBackupReminder('acc-a');
  expect(sendNotification).toHaveBeenCalledTimes(1);
  expect(useSettingsStore.getState().settings.lastBackupReminderAt).toBe(100);
  expect(useUiStore.getState().toasts).toHaveLength(1);
  expect(useUiStore.getState().toasts[0].type).toBe('warning');
  expect(logger.warn).toHaveBeenCalledWith('[notification] Failed to persist backup reminder time');
  expect(logger.warn).not.toHaveBeenCalledWith(
    '[notification] Backup reminder check failed:',
    expect.anything(),
  );
});
