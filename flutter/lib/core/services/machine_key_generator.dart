import 'package:uuid/uuid.dart';

/// 机器 key 生成器。
///
/// 为自定义字段生成全局唯一的机器可读标识符。
/// 格式：`auto_{uuid_v4前8位}`，如 `auto_a3f7d2e1`。
///
/// 设计原则：
/// - `auto_` 前缀明确标识机器生成，天然不与预定义字段冲突
/// - UUID 前 8 位提供 2^32 级别的唯一性
/// - key 完全无意义，不透露字段信息
/// - 全局唯一，跨 section 引用也不会冲突
class MachineKeyGenerator {
  static const _uuid = Uuid();

  /// 生成新的机器 key。
  ///
  /// 格式：`auto_{uuid_v4前8位}`
  /// 示例：`auto_a3f7d2e1`
  static String generate() {
    final uuid = _uuid.v4();
    return 'auto_${uuid.substring(0, 8)}';
  }

  /// 检查一个 key 是否是机器生成的 key。
  static bool isAutoKey(String key) {
    return key.startsWith('auto_');
  }

  /// 验证 key 格式是否合法。
  static bool isValid(String key) {
    if (key.isEmpty || key.length > 13) return false;
    return RegExp(r'^[a-z0-9_]+$').hasMatch(key);
  }
}
