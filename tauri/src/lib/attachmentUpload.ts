import { invoke } from '@tauri-apps/api/core';
import { cleanupStagedFile, isUriPath, stageFileForUpload } from './mobileFileTransfer';

/** 判断是否为 Android content:// URI */
function isContentUri(path: string): boolean {
  return path.startsWith('content://');
}

/** MIME 类型映射表（扩展名 → MIME type） */
export const MIME_MAP: Record<string, string> = {
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  png: 'image/png',
  gif: 'image/gif',
  webp: 'image/webp',
  svg: 'image/svg+xml',
  pdf: 'application/pdf',
  txt: 'text/plain',
  md: 'text/markdown',
  json: 'application/json',
  xml: 'application/xml',
  csv: 'text/csv',
};

/** 从文件路径或 URI 中提取文件名 */
export function getFileName(filePath: string): string {
  if (!filePath) return 'file';

  try {
    // 对 content:// URI 先去掉 query 与 fragment，只保留 path 部分
    if (filePath.startsWith('content://')) {
      const withoutQuery = filePath.split('?')[0].split('#')[0];
      // 某些 Provider 的 path 形如 /document/primary:Download/file.pdf 或 /raw:/path/file.pdf
      const segments = withoutQuery.split('/');
      for (let i = segments.length - 1; i >= 0; i--) {
        const seg = decodeURIComponent(segments[i]);
        const cleaned = seg.replace(/^(raw:|document:|primary:|msf:|msd:)/, '');
        if (cleaned && cleaned !== 'content:' && cleaned.includes('.')) {
          return cleaned;
        }
      }
    }
  } catch {
    // 忽略 URI 解析异常，继续兜底
  }

  return filePath.split('/').pop() || filePath.split('\\').pop() || 'file';
}

/** 从文件名获取 MIME type */
export function getMimeType(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() || '';
  return MIME_MAP[ext] || 'application/octet-stream';
}

/** 获取文件大小（字节），失败时返回 0 */
export async function getFileSize(filePath: string): Promise<number> {
  return invoke<number>('fs_get_file_size', { path: filePath }).catch(() => 0);
}

/**
 * 检查指定路径是否为目录。
 * 通过 Tauri 命令 fs_is_dir 查询文件系统元数据。
 */
export async function checkPathIsDir(filePath: string): Promise<boolean> {
  return invoke<boolean>('fs_is_dir', { path: filePath }).catch(() => false);
}

/**
 * 将路径数组过滤为文件和目录两组。
 * 所有检测并行执行后统一分类。
 *
 * @returns { files: 文件路径数组, dirs: 目录路径数组 }
 */
export async function filterOutDirectories(
  paths: string[],
): Promise<{ files: string[]; dirs: string[] }> {
  if (paths.length === 0) return { files: [], dirs: [] };

  // 并行检测所有路径
  const results = await Promise.allSettled(
    paths.map(async (p) => ({ path: p, isDir: await checkPathIsDir(p) })),
  );

  const files: string[] = [];
  const dirs: string[] = [];

  for (const r of results) {
    if (r.status === 'rejected') {
      // 检测失败时保守处理，当作文件让其正常走上传（后端会报错）
      files.push((r.reason as { path?: string })?.path || '');
      continue;
    }
    if (r.value.isDir) {
      dirs.push(r.value.path);
    } else {
      files.push(r.value.path);
    }
  }

  return { files, dirs };
}

/** 读取文件对话框，返回选中的文件路径或 null */
export async function pickFileToAttach(): Promise<string | null> {
  try {
    const { openWithPause } = await import('@/lib/dialog');
    const filePath = await openWithPause({ multiple: false, title: 'Select file to attach' });
    if (filePath && typeof filePath === 'string') return filePath;
    return null;
  } catch {
    return null;
  }
}

/**
 * 上传单个附件到指定对象。
 *
 * 完整流程：解析文件名 → 获取文件大小 → 生成 UUID → 复制到 Vault → 写入数据库。
 *
 * @param filePath  - 源文件的绝对路径或 URI
 * @param objectId  - 目标对象 ID
 * @returns 新创建的附件 ID
 */
export async function uploadSingleAttachment(filePath: string, objectId: string): Promise<string> {
  const fileNameFromPath = getFileName(filePath);
  let fileName = fileNameFromPath;
  const id = crypto.randomUUID();

  let uploadPath = filePath;
  let sizeBytes: number;
  let stagedPath: string | null = null;
  let vaultPath: string;

  if (isContentUri(filePath)) {
    // Android 上 plugin-dialog 返回 content:// URI，通过原生插件直接流式导入 Vault。
    const imported = await invoke<{
      vaultPath: string;
      sizeBytes: number;
      displayName?: string;
    }>('attachment_import_content_uri', {
      objectId,
      attachmentId: id,
      contentUri: filePath,
      fileName,
    });
    vaultPath = imported.vaultPath;
    sizeBytes = imported.sizeBytes;
    // Android content URI 的真实文件名需以 Kotlin 端 ContentResolver 查询结果为准
    const realName = imported.displayName || getFileName(imported.vaultPath);
    if (realName) {
      fileName = realName;
    }
  } else {
    // 桌面端或 file:// URI：先中转/获取大小，再复制到 Vault。
    if (isUriPath(filePath)) {
      const staged = await stageFileForUpload(filePath);
      uploadPath = staged.localPath;
      sizeBytes = staged.size;
      stagedPath = staged.localPath;
    } else {
      sizeBytes = await getFileSize(filePath);
    }

    vaultPath = await invoke<string>('attachment_copy_to_vault', {
      srcPath: uploadPath,
      objectId: objectId,
      attachmentId: id,
      fileName: fileName,
    }).catch(() => uploadPath);

    if (stagedPath) {
      await cleanupStagedFile(stagedPath);
    }
  }

  await invoke('attachment_save', {
    objectId: objectId,
    meta: {
      id,
      objectId,
      fileName,
      mimeType: getMimeType(fileName),
      sizeBytes,
      createdAt: new Date().toISOString(),
      srcPath: filePath,
      vaultPath,
    },
  });

  return id;
}

/**
 * 依次上传多个附件，每完成一个调用 onProgress。
 *
 * @param paths     - 源文件路径数组
 * @param objectId  - 目标对象 ID
 * @param onProgress - 每完成一个文件后的回调 (currentIndex, total, fileName)
 */
export async function uploadAttachmentsSequentially(
  paths: string[],
  objectId: string,
  onProgress?: (currentIndex: number, total: number, fileName: string) => void,
): Promise<void> {
  for (let i = 0; i < paths.length; i++) {
    const fileName = getFileName(paths[i]);
    onProgress?.(i, paths.length, fileName);
    await uploadSingleAttachment(paths[i], objectId);
  }
}
