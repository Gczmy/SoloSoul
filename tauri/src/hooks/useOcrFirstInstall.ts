import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { confirmWithPause } from '@/lib/dialog';
import {
  useOcrInstallStore,
  isOcrFirstInstallDone,
  markOcrFirstInstallDone,
} from '@/stores/ocrInstallStore';
import { isMobilePlatformSync } from '@/lib/platform';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { logger } from '@/lib/logger';
import type { OcrInstallPhase } from '@/components/ui/OcrInstallBanner';
import type { OcrModelStatus } from '@/lib/ipc';

/**
 * P041: 从 AppRoutes 拆出的 OCR 首装逻辑——首次启动静默安装 bundled small 模型，
 * 并在安装期间拦截窗口关闭（桌面端），提示用户避免退出导致安装不完整。
 */
export function useOcrFirstInstall() {
  const isMobilePlatform = isMobilePlatformSync();
  const { t } = useTranslation(['ocr', 'settings']);
  const [showOcrBanner, setShowOcrBanner] = useState(false);
  const { isInstalling, progress, error, startListening } = useOcrInstallStore();

  // Derive OCR banner phase from store state for the new banner component.
  const ocrPhase: OcrInstallPhase = error ? 'error' : isInstalling ? 'installing' : 'completed';

  // 首次启动时静默安装 bundled small OCR 模型（桌面端）
  const triggerOcrFirstInstall = useCallback(async () => {
    if (isMobilePlatform) {
      markOcrFirstInstallDone();
      return;
    }
    if (isOcrFirstInstallDone()) return;
    try {
      const status = await invoke<OcrModelStatus>('ocr_get_model_status', { tier: 'small' });
      if (status.installed) {
        markOcrFirstInstallDone();
        return;
      }
      if (!status.bundled) {
        // 安装包未附带 small 模型，跳过自动安装。
        markOcrFirstInstallDone();
        return;
      }
      setShowOcrBanner(true);
      startListening();
      await invoke<void>('ocr_install_bundled_model_with_progress', { tier: 'small' });
    } catch {
      // 错误会通过 ocr-install-progress 事件进入 store；这里兜底确保 banner 不消失。
      setShowOcrBanner(true);
    }
  }, [startListening, isMobilePlatform]);

  useEffect(() => {
    triggerOcrFirstInstall();
  }, [triggerOcrFirstInstall]);

  // OCR 模型安装期间拦截窗口关闭，提示用户避免退出导致安装不完整（桌面端）
  useEffect(() => {
    if (isMobilePlatform || !isInstalling) return;

    const appWindow = getCurrentWindow();
    let unlisten: (() => void) | undefined;

    appWindow
      .onCloseRequested(async (event) => {
        event.preventDefault();
        const confirmed = await confirmWithPause(t('quit_while_installing_message'), {
          title: t('quit_while_installing_title'),
          kind: 'warning',
        });
        if (confirmed) {
          await appWindow.close();
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((err) => logger.warn('[useOcrFirstInstall] CloseRequested listener failed:', err));

    return () => {
      unlisten?.();
    };
  }, [isInstalling, t, isMobilePlatform]);

  const retryOcrInstall = useCallback(() => {
    useOcrInstallStore.getState().reset();
    triggerOcrFirstInstall();
  }, [triggerOcrFirstInstall]);

  const closeOcrBanner = useCallback(() => {
    setShowOcrBanner(false);
    markOcrFirstInstallDone();
  }, []);

  return { showOcrBanner, ocrPhase, progress, error, retryOcrInstall, closeOcrBanner };
}
