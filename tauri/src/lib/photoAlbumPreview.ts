import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { AttachmentItem } from '@/lib/attachmentUtils';

/** 简单 LRU 有界缓存：超过容量时淘汰最久未用的条目。 */
function createBoundedCache<T>(capacity: number): {
  get: (key: string) => T | undefined;
  set: (key: string, value: T) => void;
} {
  const map = new Map<string, T>();
  return {
    get(key) {
      const v = map.get(key);
      if (v !== undefined) {
        // 命中即刷新 LRU 顺序
        map.delete(key);
        map.set(key, v);
      }
      return v;
    },
    set(key, value) {
      if (map.has(key)) map.delete(key);
      map.set(key, value);
      while (map.size > capacity) {
        const oldest = map.keys().next().value as string | undefined;
        if (oldest === undefined) break;
        map.delete(oldest);
      }
    },
  };
}

/**
 * 照片集预览加载器（P024 附件照片集方案 §3.1）。
 *
 * - 网格缩略图：`fs_read_image_preview`（Rust 端解码 → 缩放 → JPEG 重编码），
 *   避免整文件 base64（比原文件大 ~33%）驻留 JS 堆——手机原图普遍超 10 MiB
 *   data URL 上限，整图方案在网格场景必然失败/爆内存。
 * - 全屏预览：小图（≤10 MiB）直接走 `fs_read_file_as_data_url` 保留原始质量
 *   （PNG 透明等）；超限时回退 `fs_read_image_preview` 缩放。
 * - 两级尺寸约定：网格 ≈256px（每张几十 KB），全屏 ≈1600px。
 * - 内存缓存（Map）+ 并发去重：同一附件避免重复 IPC 与重复解码；磁盘缓存列为后续优化。
 */

/** 网格缩略图最长边上限（像素）。 */
export const THUMB_MAX_DIM = 256;
/** 全屏预览最长边上限（像素）——仅当原图超 data URL 上限时缩放。 */
export const FULL_PREVIEW_MAX_DIM = 1600;

function isUriPath(path: string): boolean {
  return path.startsWith('content://') || path.startsWith('file://');
}

/** 安全约束：只用 `item.vaultPath`；空路径或 content:// / file:// URI 直接拒绝（与预览遮罩一致）。 */
function assertVaultPath(item: AttachmentItem): string {
  const filePath = item.vaultPath;
  if (!filePath || isUriPath(filePath)) {
    throw new Error('Attachment is not stored in vault');
  }
  return filePath;
}

/**
 * 缩略图内存缓存：`${vaultPath}::${maxDim}` → data URL。
 * 256px 缩略图每张仅几十 KB，容量 256 张上限（≈ 十 MB 级）足够一次相册浏览；
 * 超出即按 LRU 淘汰，避免长时间浏览后无限增长。
 */
const thumbCache = createBoundedCache<string>(256);
const thumbInflight = new Map<string, Promise<string>>();

/**
 * 全屏预览内存缓存：`vaultPath` → data URL（原始或缩放后）。
 * 整图 data URL 可达 ~13 MB（10 MiB 文件 base64 放大 1/3），**必须**有界——
 * 设计文档 §3.3「仅缓存当前 ±1 张」；容量 6 覆盖当前 ±1 与滑动往返，超出 LRU 淘汰。
 */
const fullCache = createBoundedCache<string>(6);
const fullInflight = new Map<string, Promise<string>>();

/** 加载网格缩略图（Rust 缩放，data:image/jpeg）。 */
export async function loadThumbnailUrl(item: AttachmentItem): Promise<string> {
  const filePath = assertVaultPath(item);
  const key = `${filePath}::${THUMB_MAX_DIM}`;
  const cached = thumbCache.get(key);
  if (cached) return Promise.resolve(cached);
  const inflight = thumbInflight.get(key);
  if (inflight) return inflight;
  const p = invoke<string>('fs_read_image_preview', { path: filePath, maxDim: THUMB_MAX_DIM })
    .catch((err: unknown) => {
      // `image` crate 无法解码 SVG（且 SVG 是纯文本小文件）——回退整图 data URL
      if (
        item.fileName.toLowerCase().endsWith('.svg') ||
        item.mimeType === 'image/svg+xml'
      ) {
        return invoke<string>('fs_read_file_as_data_url', { path: filePath });
      }
      throw err;
    })
    .then((url) => {
      thumbCache.set(key, url);
      return url;
    })
    .finally(() => {
      thumbInflight.delete(key);
    });
  thumbInflight.set(key, p);
  return p;
}

/**
 * 加载全屏预览：先尝试整图 data URL（小图保质量）；失败（超 10 MiB / 解码问题）
 * 时回退到 Rust 端缩放重编码。并发请求去重。
 */
export async function loadFullPreviewUrl(item: AttachmentItem): Promise<string> {
  const filePath = assertVaultPath(item);
  const cached = fullCache.get(filePath);
  if (cached) return Promise.resolve(cached);
  const inflight = fullInflight.get(filePath);
  if (inflight) return inflight;
  const p = (async () => {
    try {
      const raw = await invoke<string>('fs_read_file_as_data_url', { path: filePath });
      if (!raw.startsWith('data:')) throw new Error('Unexpected data URL payload');
      return raw;
    } catch {
      // 超过 data URL 上限（手机原图常见）→ Rust 端缩放重编码
      const scaled = await invoke<string>('fs_read_image_preview', {
        path: filePath,
        maxDim: FULL_PREVIEW_MAX_DIM,
      });
      if (!scaled.startsWith('data:image/jpeg;base64,')) {
        throw new Error('Unexpected preview payload');
      }
      return scaled;
    }
  })().then((url) => {
    fullCache.set(filePath, url);
    return url;
  }).finally(() => {
    fullInflight.delete(filePath);
  });
  fullInflight.set(filePath, p);
  return p;
}
