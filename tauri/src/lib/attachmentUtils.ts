import type { CSSProperties } from 'react';

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

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ── Button style helpers ─────────────────────────────────────

export const pgBtn: CSSProperties = {
  width: 30,
  height: 30,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  border: 'none',
  borderRadius: 6,
  background: 'transparent',
  cursor: 'pointer',
  color: 'var(--text-secondary)',
  fontSize: 14,
  transition: 'background 0.15s, color 0.15s',
};

export const miniBtn: CSSProperties = {
  width: 28,
  height: 28,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  border: 'none',
  borderRadius: 6,
  background: 'transparent',
  cursor: 'pointer',
  fontSize: 12,
  color: 'var(--text-secondary)',
  transition: 'background 0.15s, color 0.15s',
};

export function btnHoverEnter(e: React.MouseEvent<HTMLButtonElement>) {
  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
  e.currentTarget.style.color = 'var(--accent-primary)';
}

export function btnHoverLeave(e: React.MouseEvent<HTMLButtonElement>) {
  e.currentTarget.style.background = 'transparent';
  e.currentTarget.style.color = 'var(--text-secondary)';
}

export function btnDelEnter(e: React.MouseEvent<HTMLButtonElement>) {
  e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 12%, transparent)';
}

export function btnDelLeave(e: React.MouseEvent<HTMLButtonElement>) {
  e.currentTarget.style.background = 'transparent';
}

// ── MIME helpers ──────────────────────────────────────────────

export function isImageMime(mimeType: string, fileName: string): boolean {
  const ext = fileName.split('.').pop()?.toLowerCase() || '';
  return mimeType.startsWith('image/') || ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext);
}

export function previewItemByMime(item: AttachmentItem): 'image' | 'pdf' | 'text' | 'other' {
  const ext = item.fileName.split('.').pop()?.toLowerCase() || '';
  if (item.mimeType.startsWith('image/') || ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext)) return 'image';
  if (item.mimeType === 'application/pdf' || ext === 'pdf') return 'pdf';
  if (item.mimeType.startsWith('text/') || ['json', 'xml', 'csv', 'md', 'txt'].includes(ext)) return 'text';
  return 'other';
}
