import { useCallback, useEffect, useRef, useState } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { isMacOSSync } from '@/lib/platform';
import { usePrefetchData } from '@/lib/prefetch/usePrefetchData';
import { prefetchRegistry } from '@/lib/prefetch/registry';
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
 *
 * Prefetch Runtime（P?）：tier/status 数据改由 `prefetchRegistry.ocrModel`
 * 提供——登录/解锁后后台预热，进入页面命中缓存直接渲染（无骨架期）；
 * 安装/下载/删除成功后 reload() 刷新缓存。
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
  const {
    data,
    loading: storeLoading,
    error,
    reload,
  } = usePrefetchData(prefetchRegistry.ocrModel, {
    enabled,
  });
  // 乐观档位：切换成功后立即生效（权威值以 ocr_get_active_tier 为准，下次刷新回归后端值）
  const [optimisticTier, setOptimisticTier] = useState<string | null>(null);
  // P133: macOS 默认 Vision 引擎（缓存未就绪时兜底；权威值以 ocr_get_active_tier 为准）。
  const activeTier = optimisticTier ?? data?.activeTier ?? (isMacOSSync() ? 'vision' : 'small');
  const tiers: OcrTierInfo[] = data?.tiers ?? [];
  const statusMap: Record<string, OcrModelStatus> = data?.statusMap ?? {};
  // 数据未就绪且未失败即视为加载中（与旧「初始 loading=true」语义一致，
  // 保证骨架在首帧即显示；预热完成后 data 就绪 → 直接渲染内容，无骨架期）。
  // enabled=false（移动端 Settings）时不显示加载态。
  const loading = enabled && (storeLoading || (data === null && error === null));

  // 加载失败反馈：store 吞错（warmup/缓存语义），此处补 toast（error 变化去重）
  const prevErrorRef = useRef<string | null>(null);
  useEffect(() => {
    if (error && error !== prevErrorRef.current) {
      prevErrorRef.current = error;
      onError(new Error(error), t('ocr:load_status_failed'));
    } else if (!error) {
      prevErrorRef.current = null;
    }
  }, [error, onError, t]);

  const [installingTier, setInstallingTier] = useState<string | null>(null);
  const [downloadingTier, setDownloadingTier] = useState<string | null>(null);
  const [deletingTier, setDeletingTier] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState('');

  const handleTierChange = useCallback(
    async (tier: string) => {
      try {
        await invoke<void>('ocr_set_active_tier', { tier });
        setOptimisticTier(tier);
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
        await reload();
        onInstallSuccess?.(t('ocr:install_success', { tier }));
      } catch (e) {
        onError(e, t('ocr:install_failed', { tier }));
      } finally {
        setInstallingTier(null);
      }
    },
    [t, onError, onInstallSuccess, reload],
  );

  const handleDelete = useCallback(
    async (tier: string) => {
      setDeletingTier(tier);
      try {
        await invoke<void>('ocr_delete_model', { tier });
        await reload();
        onDeleteSuccess?.(t('ocr:delete_success', { tier }));
      } catch (e) {
        onError(e, t('ocr:delete_failed', { tier }));
      } finally {
        setDeletingTier(null);
      }
    },
    [t, onError, onDeleteSuccess, reload],
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
          await reload();
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
    [downloadUrl, t, onError, onDownloadSuccess, reload, confirmDownload],
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
