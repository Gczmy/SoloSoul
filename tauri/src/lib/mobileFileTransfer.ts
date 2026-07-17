/**
 * 移动端文件 URI 中转工具
 *
 * Android 上 `plugin-dialog` 返回的是 `content://` URI，Rust 标准库无法直接读写。
 * 这里借助 `tauri-apps/plugin-fs`（其底层已适配 content URI）做中转：
 * - 上传：把 content URI 复制到应用缓存，拿到本地路径和大小后再交给 Rust。
 * - 下载/导出：先让 Rust 写到应用缓存的临时文件，再用 plugin-fs 复制到目标 URI。
 */

import { invoke } from '@tauri-apps/api/core';
import { copyFile, mkdir, remove, stat } from '@tauri-apps/plugin-fs';
import { appCacheDir, join } from '@tauri-apps/api/path';

const STAGE_DIR = 'solosoul_mobile_stage';

function generateId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
}

export function isUriPath(path: string): boolean {
  return path.startsWith('content://') || path.startsWith('file://');
}

async function ensureStageDir(): Promise<string> {
  const cacheDir = await appCacheDir();
  const stageDir = await join(cacheDir, STAGE_DIR);
  await mkdir(stageDir, { recursive: true });
  return stageDir;
}

function getFileName(path: string): string {
  return path.split('/').pop() || path.split('\\').pop() || 'file';
}

/**
 * 将 content:// 等 URI 复制到应用缓存，返回本地绝对路径和文件大小。
 * 如果传入的已经是本地路径，则原样返回（仅统计大小）。
 */
export async function stageFileForUpload(
  sourcePath: string,
): Promise<{ localPath: string; size: number }> {
  if (!isUriPath(sourcePath)) {
    const info = await stat(sourcePath).catch(() => null);
    return { localPath: sourcePath, size: info?.size ?? 0 };
  }

  const stageDir = await ensureStageDir();
  const name = `${generateId()}_${getFileName(sourcePath)}`;
  const localPath = await join(stageDir, name);
  await copyFile(sourcePath, localPath);
  const info = await stat(localPath);
  return { localPath, size: info.size };
}

/**
 * 为“保存到系统文件选择器返回的 URI”生成一个临时缓存路径。
 * 调用方应让 Rust 把内容写入该路径，然后调用 `copyStagedFileToDest` 复制到 URI。
 */
export async function prepareStagedDownloadPath(fileName: string): Promise<string> {
  const stageDir = await ensureStageDir();
  const name = `${generateId()}_${getFileName(fileName)}`;
  return join(stageDir, name);
}

/**
 * 把本地临时文件复制到目标 URI（通常是 Android content:// URI）。
 */
export async function copyStagedFileToDest(stagedPath: string, destUri: string): Promise<void> {
  await copyFile(stagedPath, destUri);
}

/**
 * 把 Vault 中的文件下载到目标路径。
 * 桌面端直接调用 downloadFn；Android 端若目标是 content:// URI，
 * 则通过原生插件直接把 Vault 文件流式复制到 URI。
 */
export async function downloadViaStage(
  srcPath: string,
  destPath: string,
  fileName: string,
  downloadFn: (src: string, dest: string) => Promise<void>,
): Promise<void> {
  if (!srcPath) {
    throw new Error('Attachment source path is missing');
  }
  if (isUriPath(srcPath)) {
    throw new Error(
      'Attachment is not stored in vault (source is still a content URI). Please re-upload.',
    );
  }

  if (!isUriPath(destPath)) {
    await downloadFn(srcPath, destPath);
    return;
  }

  // Android content:// URI：原生插件直接处理，避免 plugin-fs 无法复制 URI。
  if (destPath.startsWith('content://')) {
    await invoke('attachment_export_content_uri', {
      srcPath: srcPath,
      destUri: destPath,
    });
    return;
  }

  // file:// URI 等兜底：先中转缓存，再用 plugin-fs 复制。
  const stagedPath = await prepareStagedDownloadPath(fileName);
  try {
    await downloadFn(srcPath, stagedPath);
    await copyStagedFileToDest(stagedPath, destPath);
  } finally {
    await cleanupStagedFile(stagedPath);
  }
}

/**
 * 将外部导入包（content:// URI 或本地路径）复制到应用缓存，返回本地绝对路径。
 */
export async function stageImportPackage(sourcePath: string): Promise<string> {
  if (!isUriPath(sourcePath)) {
    return sourcePath;
  }
  const stageDir = await ensureStageDir();
  const name = `${generateId()}_import.solosoul`;
  const localPath = await join(stageDir, name);
  await copyFile(sourcePath, localPath);
  return localPath;
}

/**
 * 清理单个临时文件（忽略错误）。
 */
export async function cleanupStagedFile(stagedPath: string): Promise<void> {
  try {
    await remove(stagedPath);
  } catch {
    // 忽略清理失败
  }
}
