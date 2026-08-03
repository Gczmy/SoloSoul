import { useCallback, useEffect, useState } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { isMacOSSync } from '@/lib/platform';
import type { OcrTierInfo, OcrModelStatus } from '@/lib/ipc';

/**
 * OCR 模型管理共享逻辑（P140）。
 *
 * `OcrSettingsPage` 与 `OcrPage` 此前各自实现一套逐字重复的
 * tier/status 加载、安装/下载/删除处理（约 70 行）。此处统一收敛，
 * 页面差异通过可选回调注入：
 * - `onError`：所有操作失败的统一错误提示（必传，来自 useToastError）
 * - `onTierChangeSuccess` / `onInstallSuccess` / `onDownloadSuccess` /
 *   `onDeleteSuccess`：各操作成功的可选提示（不传则静默成功）
 * - `confirmDownload`：下载前的确认包装（Settings 页用 requestConfirm
 *   弹确认框；OcrPage 直接下载，不传）
 */
export interface UseOcrModelManagerOptions {
  /** 挂载时是否加载 tier 列表与状态。Settings 页移动端（ML Kit）传 false。 */
  enabled?: boolean;
  t: TFunction;
  /** 统一错误提示（useToastError.onError）。所有失败场景都传入上下文文案。 */
  onError: (err: unknown, context: string) => void;
  onTierChangeSuccess?: (message: string) => void;
  onInstallSuccess?: (message: string) => void;
  onDownloadSuccess?: (message: string) => void;
  onDeleteSuccess?: (message: string) => void;
  /** 下载确认回调：传入后下载前先确认；不传则直接下载。 */
  confirmDownload?: (params: {
    tier: string;
    size: string;
    message: string;
    confirmLabel: string;
    cancelLabel: string;
    onConfirm: () => void;
  }) => void;
}

export function useOcrModelManager({
  enabled = true,
  t,
  onError,
  onTierChangeSuccess,
  onInstallSuccess,
  onDownloadSuccess,
  onDeleteSuccess,
  confirmDownload,
}: UseOcrModelManagerOptions) {
  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  // P133: macOS 默认 Vision 引擎（后端加载前兜底；权威值以 ocr_get_active_tier 为准）。
  const [activeTier, setActiveTier] = useState(() => (isMacOSSync() ? 'vision' : 'small'));
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loading, setLoading] = useState(enabled);
  const [installingTier, setInstallingTier] = useState<string | null>(null);
  const [downloadingTier, setDownloadingTier] = useState<string | null>(null);
  const [deletingTier, setDeletingTier] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState('');

  const loadTiersAndStatus = useCallback(async () => {
    try {
      setLoading(true);
      const [tierList, currentTier] = await Promise.all([
        invoke<OcrTierInfo[]>('ocr_list_available_tiers'),
        invoke<string>('ocr_get_active_tier'),
      ]);
      setTiers(tierList);
      setActiveTier(currentTier);

      const statuses: Record<string, OcrModelStatus> = {};
      await Promise.all(
        tierList.map(async (tier) => {
          const status = await invoke<OcrModelStatus>('ocr_get_model_status', { tier: tier.tier });
          statuses[tier.tier] = status;
        }),
      );
      setStatusMap(statuses);
    } catch (e) {
      onError(e, t('ocr:load_status_failed'));
    } finally {
      setLoading(false);
    }
    // onError/t 为稳定引用（useToastError useCallback / i18next），省略以保持 mount-only 语义
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 仅在 enabled 变化时（如平台切换）重新加载；loadTiersAndStatus 为稳定 useCallback（空依赖）
  useEffect(() => {
    if (enabled) {
      loadTiersAndStatus();
    }
  }, [enabled, loadTiersAndStatus]);

  const handleTierChange = useCallback(
    async (tier: string) => {
      try {
        await invoke<void>('ocr_set_active_tier', { tier });
        setActiveTier(tier);
        onTierChangeSuccess?.(t('ocr:set_tier_success', { tier }));
      } catch (e) {
        onError(e, t('ocr:set_tier_failed'));
      }
    },
    [t, onError, onTierChangeSuccess],
  );

  const handleInstallBundled = useCallback(
    async (tier: string) => {
      setInstallingTier(tier);
      try {
        await invoke<void>('ocr_install_bundled_model', { tier });
        await loadTiersAndStatus();
        onInstallSuccess?.(t('ocr:install_success', { tier }));
      } catch (e) {
        onError(e, t('ocr:install_failed', { tier }));
      } finally {
        setInstallingTier(null);
      }
    },
    [t, onError, onInstallSuccess, loadTiersAndStatus],
  );

  const handleDelete = useCallback(
    async (tier: string) => {
      setDeletingTier(tier);
      try {
        await invoke<void>('ocr_delete_model', { tier });
        await loadTiersAndStatus();
        onDeleteSuccess?.(t('ocr:delete_success', { tier }));
      } catch (e) {
        onError(e, t('ocr:delete_failed', { tier }));
      } finally {
        setDeletingTier(null);
      }
    },
    [t, onError, onDeleteSuccess, loadTiersAndStatus],
  );

  const handleDownload = useCallback(
    async (tier: string) => {
      if (!downloadUrl.trim()) {
        onError(new Error(t('ocr:download_url_required')), t('ocr:download_url_required'));
        return;
      }
      const size = tier === 'tiny' ? '1.5MB' : tier === 'medium' ? '132MB' : '30MB';
      const runDownload = async () => {
        setDownloadingTier(tier);
        try {
          await invoke<void>('ocr_download_model', { tier, baseUrl: downloadUrl.trim() });
          await loadTiersAndStatus();
          onDownloadSuccess?.(t('ocr:download_success', { tier }));
        } catch (e) {
          onError(e, t('ocr:download_failed', { tier }));
        } finally {
          setDownloadingTier(null);
        }
      };
      if (confirmDownload) {
        confirmDownload({
          tier,
          size,
          message: t('ocr:confirm_download_message', { tier, size }),
          confirmLabel: t('ocr:confirm_download_ok'),
          cancelLabel: t('common:cancel'),
          onConfirm: runDownload,
        });
      } else {
        await runDownload();
      }
    },
    [downloadUrl, t, onError, onDownloadSuccess, loadTiersAndStatus, confirmDownload],
  );

  return {
    tiers,
    activeTier,
    statusMap,
    loading,
    installingTier,
    downloadingTier,
    deletingTier,
    downloadUrl,
    setDownloadUrl,
    handleTierChange,
    handleInstallBundled,
    handleDelete,
    handleDownload,
  };
}
