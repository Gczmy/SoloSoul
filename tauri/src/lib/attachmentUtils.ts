// ── Types ────────────────────────────────────────────────────

export interface AttachmentItem {
  id: string;
  objectId: string;
  fileName: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
  deletedAt?: string | null;
  srcPath?: string | null;
  vaultPath?: string | null;
}

// ── Formatting ────────────────────────────────────────────────

/** Truncate a file name preserving its extension: "abcdefg…-.pdf" instead of "abcdefg…" */
export function truncateFileName(fileName: string, maxLen = 28): string {
  const dotIndex = fileName.lastIndexOf('.');
  if (dotIndex <= 0) {
    if (fileName.length <= maxLen) return fileName;
    return fileName.slice(0, maxLen - 1) + '…';
  }
  const baseName = fileName.slice(0, dotIndex);
  const ext = fileName.slice(dotIndex);
  if (fileName.length <= maxLen) return fileName;
  const available = maxLen - ext.length - 2;
  if (available <= 1) return fileName.slice(0, maxLen - 1) + '…';
  return baseName.slice(0, available) + '…-' + ext;
}

// ── Download helper ───────────────────────────────────────────

/**
 * P011: 附件下载统一入口——AttachmentViewer 与 useAttachmentManager 的
 * `handleDownload` 曾逐字符重复（动态 import dialog + saveWithPause + downloadViaStage
 * + 成功/失败 toast 文案）。依赖经参数注入保持纯函数可测。
 *
 * @returns true 表示已保存到目标位置；false 表示用户取消（无目标路径）。
 */
export async function downloadAttachmentFile(params: {
  filePath: string;
  fileName: string;
  invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
  showToast: (opts: { type: 'success' | 'error'; message: string }) => void;
  t: (key: string, opts?: Record<string, unknown>) => string;
  downloadViaStage: (
    src: string,
    dest: string,
    fileName: string,
    downloadFn: (src: string, dest: string) => Promise<void>,
  ) => Promise<void>;
}): Promise<boolean> {
  const { filePath, fileName, invoke, showToast, t, downloadViaStage } = params;
  try {
    const { saveWithPause } = await import('@/lib/dialog');
    const destPath = await saveWithPause({
      defaultPath: fileName,
    });
    if (!destPath) return false;
    await downloadViaStage(filePath, destPath, fileName, async (src, dest) => {
      await invoke('attachment_download', { srcPath: src, destPath: dest });
    });
    showToast({
      type: 'success',
      message: t('common:download_result', { defaultValue: 'Downloaded successfully' }),
    });
    return true;
  } catch (e) {
    showToast({ type: 'error', message: `${t('common:download_failed')}: ${e}` });
    return false;
  }
}

// ── MIME helpers ──────────────────────────────────────────────

export function previewItemByMime(item: AttachmentItem): 'image' | 'pdf' | 'text' | 'other' {
  const ext = item.fileName.split('.').pop()?.toLowerCase() || '';
  if (
    item.mimeType.startsWith('image/') ||
    ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext)
  )
    return 'image';
  if (item.mimeType === 'application/pdf' || ext === 'pdf') return 'pdf';
  if (item.mimeType.startsWith('text/') || ['json', 'xml', 'csv', 'md', 'txt'].includes(ext))
    return 'text';
  return 'other';
}
