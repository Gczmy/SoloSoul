/// MRZ 日期规范化工具
///
/// MRZ（机读区）日期格式为 6 位数字字符串 `YYMMDD`。
/// 本工具将其转换为 ISO 8601 标准日期格式 `YYYY-MM-DD`。
///
/// 世纪判断规则（ICAO Doc 9303 行业惯例）：
/// - YY >= 50 → 19YY（如 99 → 1999）
/// - YY < 50 → 20YY（如 01 → 2001）
library mrz_date_utils;

/// 将 MRZ 6 位日期字符串 (YYMMDD) 解析为 ISO 8601 日期格式 (YYYY-MM-DD)。
///
/// [mrzDate] 必须为 6 位纯数字字符串，如 `"900101"`、`"251231"`。
/// 返回 `"1990-01-01"`、`"2025-12-31"` 等标准格式。
/// 若输入非法（长度不对、非数字、日期不存在），返回 `null`。
String? parseMrzDate(String mrzDate) {
  if (mrzDate.length != 6) return null;
  try {
    final year = int.parse(mrzDate.substring(0, 2));
    final month = int.parse(mrzDate.substring(2, 4));
    final day = int.parse(mrzDate.substring(4, 6));

    // 世纪判断：>=50 为 19XX，<50 为 20XX
    final fullYear = year >= 50 ? 1900 + year : 2000 + year;

    // 校验日期合法性（自动处理闰年、月份天数等）
    final dt = DateTime(fullYear, month, day);
    if (dt.year != fullYear || dt.month != month || dt.day != day) {
      return null;
    }

    return '${fullYear.toString().padLeft(4, '0')}-'
        '${month.toString().padLeft(2, '0')}-'
        '${day.toString().padLeft(2, '0')}';
  } on FormatException {
    return null;
  }
}

/// 将 ISO 日期 (YYYY-MM-DD) 格式化为友好显示格式。
///
/// 当前直接返回 ISO 字符串本身，因其已足够可读。
/// 后续如需本地化（如 "Jan 1, 1990"），可在此扩展。
String formatIsoDateForDisplay(String? isoDate) {
  if (isoDate == null || isoDate.isEmpty) return '';
  return isoDate;
}

/// 将 ISO 日期 (YYYY-MM-DD) 转换为 DateTime 对象。
///
/// 解析失败时返回 `null`。
DateTime? parseIsoDate(String isoDate) {
  try {
    return DateTime.parse(isoDate);
  } on FormatException {
    return null;
  }
}
