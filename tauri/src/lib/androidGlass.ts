import { invokeCommand } from '@/lib/ipcClient';

export const ANDROID_GLASS_MODES = ['off', 'local', 'enhanced'] as const;
export type AndroidGlassMode = (typeof ANDROID_GLASS_MODES)[number];
export const isAndroidGlassMode = (value: unknown): value is AndroidGlassMode =>
  ANDROID_GLASS_MODES.includes(value as AndroidGlassMode);

export interface AndroidGlassCapabilities {
  apiLevel: number;
  windowBlur: boolean;
  webViewVersion: string;
}

export type AndroidCreateAction = 'object' | 'page' | 'scan';
export interface AndroidGlassMenuPayload {
  requestId: string;
  title: string;
  description: string;
  closeLabel: string;
  footer: string;
  labels: Record<AndroidCreateAction, string>;
  descriptions: Record<AndroidCreateAction, string>;
  dark: boolean;
  reduceMotion: boolean;
  background: string;
  foreground: string;
  secondary: string;
  accent: string;
  container: string;
}

export interface AndroidGlassMenuResult {
  requestId: string;
  action: AndroidCreateAction | 'cancel' | 'unavailable';
}

export function closeAndroidGlassMenu(requestId: string) {
  // 锁定/卸载后的清理也必须可用，仅关闭指定请求，不读取任何账户资料。
  return invokeCommand<void>('android_close_glass_menu', { requestId }, { requireUnlocked: false });
}

/** 请求可能跨越路由/账户切换：调用方取消后，任何迟到结果都不得触发动作。 */
export function requestAndroidGlassMenu(payload: AndroidGlassMenuPayload) {
  let cancelled = false;
  let closing: Promise<void> | undefined;
  const result = invokeCommand<AndroidGlassMenuResult>('android_show_glass_menu', { payload }).then(
    (value) => {
      if (cancelled || value.requestId !== payload.requestId) return null;
      if (!['object', 'page', 'scan', 'cancel', 'unavailable'].includes(value.action)) {
        throw new Error('Invalid Android menu action');
      }
      return value.action;
    },
  );
  return {
    result,
    cancel() {
      if (closing) return closing;
      cancelled = true;
      closing = closeAndroidGlassMenu(payload.requestId).catch(() => {});
      return closing;
    },
  };
}
