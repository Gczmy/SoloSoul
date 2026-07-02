/**
 * 将字节数格式化为人类可读字符串（B / KB / MB / GB）。
 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * 从字符串中提取前缀后的内容。
 * 若字符串以指定前缀开头，返回前缀之后的部分；否则返回 null。
 */
export function tryParsePrefixedError(message: string, prefix: string): string | null {
  const idx = message.indexOf(prefix);
  if (idx === -1) return null;
  return message.slice(idx + prefix.length).trim();
}
