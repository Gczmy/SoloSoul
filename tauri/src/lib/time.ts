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
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return '刚刚';
  const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
  if (mins < 60) return rtf.format(-mins, 'minute');
  const hours = Math.floor(mins / 60);
  if (hours < 24) return rtf.format(-hours, 'hour');
  const days = Math.floor(hours / 24);
  if (days < 30) return rtf.format(-days, 'day');
  return formatTimestamp(iso).slice(0, 10);
}
