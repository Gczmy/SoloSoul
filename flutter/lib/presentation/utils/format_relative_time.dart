/// 格式化相对时间（完整格式）
/// 例如："2 year(s) ago", "3 month(s) ago", "5 day(s) ago"
String formatRelativeTime(DateTime timestamp) {
  final now = DateTime.now();
  final diff = now.difference(timestamp);

  if (diff.inDays > 365) {
    return '${(diff.inDays / 365).floor()} year(s) ago';
  } else if (diff.inDays > 30) {
    return '${(diff.inDays / 30).floor()} month(s) ago';
  } else if (diff.inDays > 0) {
    return '${diff.inDays} day(s) ago';
  } else if (diff.inHours > 0) {
    return '${diff.inHours} hour(s) ago';
  } else if (diff.inMinutes > 0) {
    return '${diff.inMinutes} minute(s) ago';
  } else {
    return 'Just now';
  }
}

/// 格式化相对时间（缩写格式）
/// 例如："2y ago", "3mo ago", "5d ago"
String formatRelativeTimeShort(DateTime timestamp) {
  final now = DateTime.now();
  final diff = now.difference(timestamp);

  if (diff.inDays > 365) {
    return '${(diff.inDays / 365).floor()}y ago';
  } else if (diff.inDays > 30) {
    return '${(diff.inDays / 30).floor()}mo ago';
  } else if (diff.inDays > 0) {
    return '${diff.inDays}d ago';
  } else if (diff.inHours > 0) {
    return '${diff.inHours}h ago';
  } else if (diff.inMinutes > 0) {
    return '${diff.inMinutes}m ago';
  } else {
    return 'Just now';
  }
}
