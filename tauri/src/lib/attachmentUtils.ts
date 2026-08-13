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
  /** 附件描述（可空；由 attachment_update_meta 维护） */
  description?: string | null;
  /** 附件标签（可空；由 attachment_update_meta 维护） */
  tags?: string[];
  /** 所属对象名称（照片集按对象分组用；对象级列表可为空） */
  objectName?: string;
  /** 所属页面 ID（照片集按对象分组时页面层级用；对象级列表可为空） */
  pageId?: string | null;
  /** 所属页面名称（照片集按对象分组时页面层级用；对象级列表可为空） */
  pageName?: string;
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

export function previewItemByMime(
  item: Pick<AttachmentItem, 'fileName' | 'mimeType'>,
): 'image' | 'pdf' | 'text' | 'other' {
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

/** 附件页面树节点（结构性类型，避免 lib → components 依赖）。 */
interface PhotoPageNode {
  pageId?: string | null;
  pageName?: string;
  objects: Array<{ objectName?: string; attachments: AttachmentItem[] }>;
}

/**
 * 从附件页面树中收集全部图片附件（照片集数据源公共过滤）。
 *
 * 首页照片集快捷入口与全局附件管理器的活跃/回收站照片集共用，
 * 防止过滤逻辑在多处复制后漂移（P044 同款收敛）。
 */
export function collectPhotoItems(pages: PhotoPageNode[] | undefined): AttachmentItem[] {
  const out: AttachmentItem[] = [];
  for (const page of pages ?? []) {
    for (const obj of page.objects) {
      for (const att of obj.attachments) {
        if (previewItemByMime(att) === 'image') {
          // 携带对象名 + 页面信息：照片集「按对象分组」的页面→对象两级结构需要
          // （对象级列表无树节点，保持为空）
          out.push({
            ...att,
            objectName: att.objectName ?? obj.objectName,
            pageId: att.pageId ?? page.pageId ?? null,
            pageName: att.pageName ?? page.pageName,
          });
        }
      }
    }
  }
  return out;
}

/**
 * 统计活跃（非回收站）附件总数：attachment_list_all 返回的 pages 树扁平计数。
 *
 * 与 collectPhotoItems 共用同一树结构；回收站附件在 trashPages 中，不计入。
 */
export function countActiveAttachments(pages: PhotoPageNode[] | undefined): number {
  return (pages ?? []).reduce(
    (n, page) => n + page.objects.reduce((m, obj) => m + obj.attachments.length, 0),
    0,
  );
}
