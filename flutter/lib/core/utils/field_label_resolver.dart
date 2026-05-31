import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';

/// 字段标签统一解析器。
///
/// 作为所有字段显示标签的**唯一入口**，消除分散在各处的硬编码映射。
/// 依赖 [SemanticTypeRegistry] 和 [AppLocalizations] 两个后端来源。
///
/// 解析优先级：
/// 1. [SemanticTypeRegistry.resolveByFieldPath] — 语义类型匹配（最精确，可区分上下文）
/// 2. [translateFieldLabel] — AppLocalizations ARB 通用翻译
/// 3. [formatFieldLabel] — 机械格式化兜底
///
/// 使用方式：
/// ```dart
/// // 在 MaterialApp 初始化后注入（仅一次）
/// FieldLabelResolver.init(AppLocalizations.of(context)!);
///
/// // 任意位置解析字段标签
/// final label = FieldLabelResolver.resolve('passport.number');
/// // → "护照号码" (zh) / "Passport Number" (en)
/// ```
class FieldLabelResolver {
  static AppLocalizations? _l10n;
  static String _languageCode = 'en';

  /// 在应用启动 / 语言切换后调用，注入当前 locale 的 AppLocalizations。
  static void init(AppLocalizations l10n) {
    _l10n = l10n;
    _languageCode = l10n.localeName;
  }

  /// 统一解析字段标签。
  ///
  /// [fieldPath] 可以是完整路径（如 `passport.number`）、数组路径（`address[0].street`）
  /// 或单段 key（如 `number`）。
  ///
  /// [sectionId] 和 [machineKey] 可选，用于从用户数据中的 `__semanticTypes`
  /// 动态查找语义类型（覆盖用户自定义字段）。
  static String resolve(
    String fieldPath, {
    String? sectionId,
    String? machineKey,
  }) {
    // 1. 语义类型查找（最精确）
    final semanticType = SemanticTypeRegistry.resolveByFieldPath(
      fieldPath,
      sectionId: sectionId,
      machineKey: machineKey,
    );
    if (semanticType != null) {
      final label = semanticType.getLabel(_languageCode);
      if (label.isNotEmpty && label != semanticType.id) return label;
    }

    // 2. ARB 通用翻译
    final lastSegment = _extractLastSegment(fieldPath);
    final l10n = _l10n;
    if (l10n != null) {
      final translated = translateFieldLabel(lastSegment, l10n);
      if (translated != formatFieldLabel(lastSegment)) return translated;
    }

    // 3. 机械格式化兜底
    return formatFieldLabel(lastSegment);
  }

  /// 提取字段路径的最后一段，同时处理数组索引。
  ///
  /// 示例：
  /// - `address[0].street` → `street`
  /// - `passport.number`   → `number`
  /// - `city`              → `city`
  static String _extractLastSegment(String fieldPath) {
    final withoutBrackets = fieldPath.replaceAll(RegExp(r'\[\d+\]'), '');
    final parts = withoutBrackets.split('.');
    return parts.isEmpty ? fieldPath : parts.last;
  }
}
