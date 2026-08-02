/**
 * 系统文件对话框封装 — 自动暂停自动锁定。
 *
 * plugin-dialog 的 `save` / `open` 会触发系统原生对话框，导致
 * visibilitychange → hidden，若用户开启「切后台锁定」则会立即锁定 Vault，
 * 中断文件选择。本封装在调用前后自动 pause() / resume() 自动锁定，
 * 消除重复的内联 pause/resume 模式。
 *
 * 约定：凡调用系统文件选择器必须使用本封装，禁止裸调 plugin-dialog。
 */
import { save as tauriSave, open as tauriOpen, confirm as tauriConfirm } from '@tauri-apps/plugin-dialog';
import type {
  SaveDialogOptions,
  OpenDialogOptions,
  ConfirmDialogOptions,
} from '@tauri-apps/plugin-dialog';

/**
 * 封装 `save()`，调用前暂停自动锁定，调用后恢复。
 * 行为与裸 `save()` 完全一致，仅增加自动锁定暂停。
 */
export async function saveWithPause(options?: SaveDialogOptions): Promise<string | null> {
  const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
    (m) => m.useAutoLockPauseStore.getState(),
  );
  pause();
  try {
    return await tauriSave(options);
  } finally {
    resume();
  }
}

/**
 * 封装 `open()`，调用前暂停自动锁定，调用后恢复。
 * 行为与裸 `open()` 完全一致，仅增加自动锁定暂停。
 */
export async function openWithPause(options?: OpenDialogOptions): Promise<string | string[] | null> {
  const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
    (m) => m.useAutoLockPauseStore.getState(),
  );
  pause();
  try {
    return await tauriOpen(options);
  } finally {
    resume();
  }
}

/**
 * 封装 `confirm()`，调用前暂停自动锁定，调用后恢复。
 * 行为与裸 `confirm()` 完全一致，仅增加自动锁定暂停。
 *
 * P130: 原生确认对话框同样触发 visibilitychange → hidden，若用户开启
 * 「切后台锁定」会中断流程或误锁 Vault，故与 open/save 同等封装。
 */
export async function confirmWithPause(
  message: string,
  options?: ConfirmDialogOptions,
): Promise<boolean> {
  const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
    (m) => m.useAutoLockPauseStore.getState(),
  );
  pause();
  try {
    return await tauriConfirm(message, options);
  } finally {
    resume();
  }
}
