// Shared time formatting utilities.

/** 使用 Intl.DateTimeFormat 格式化 ISO 时间戳为本地化日期时间字符串。 */
export function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(d);
}

/** 使用 Intl.RelativeTimeFormat 格式化相对时间（支持中文和英文等本地化输出）。 */
export function formatRelative(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
  const seconds = Math.floor(diff / 1000);
  // 对于几秒内（≈7秒），用 0 值让 rtf 输出本地化 "now"（如 "现在" / "now"）
  if (seconds < 8) return rtf.format(0, 'second');
  if (seconds < 60) return rtf.format(-seconds, 'second');
  const mins = Math.floor(seconds / 60);
  if (mins < 60) return rtf.format(-mins, 'minute');
  const hours = Math.floor(mins / 60);
  if (hours < 24) return rtf.format(-hours, 'hour');
  const days = Math.floor(hours / 24);
  if (days < 30) return rtf.format(-days, 'day');
  return formatTimestamp(iso).slice(0, 10);
}

/** 格式化 unix 秒时间戳为相对时间（本地化）；非法值返回空串。 */
export function formatRelativeFromTs(ts: number): string {
  const d = new Date(ts * 1000);
  if (Number.isNaN(d.getTime())) {
    return '';
  }
  return formatRelative(d.toISOString());
}
