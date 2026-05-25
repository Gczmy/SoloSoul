import 'dart:convert';
import 'dart:io';
import 'package:path_provider/path_provider.dart';

/// 审计事件类型。
enum AuditEventType {
  fieldSensitivityChanged,
  fieldSemanticTypeChanged,
  pluginInstallAuthorized,
  pluginRuntimeSensitivityExceeded,
  pluginFieldAccessDenied,
  pluginFieldAccessMasked,
  pluginFieldMappingChanged,
}

/// 单条审计日志记录。
class AuditLogEntry {
  final DateTime timestamp;
  final AuditEventType event;
  final String? accountId;
  final Map<String, dynamic> details;

  AuditLogEntry({
    required this.event,
    required this.details,
    this.accountId,
    DateTime? timestamp,
  }) : timestamp = timestamp ?? DateTime.now().toUtc();

  Map<String, dynamic> toJson() => {
    'timestamp': timestamp.toIso8601String(),
    'event': event.name,
    'accountId': accountId,
    'details': details,
  };

  factory AuditLogEntry.fromJson(Map<String, dynamic> json) {
    return AuditLogEntry(
      timestamp: DateTime.parse(json['timestamp'] as String),
      event: AuditEventType.values.firstWhere(
        (e) => e.name == json['event'],
        orElse: () => AuditEventType.fieldSensitivityChanged,
      ),
      accountId: json['accountId'] as String?,
      details: json['details'] as Map<String, dynamic>? ?? {},
    );
  }

  String toJsonLine() => jsonEncode(toJson());
}

/// 审计日志服务。
///
/// 采用 JSON Lines 格式存储，按日期轮转，保留 90 天。
class AuditLogService {
  static final _instance = AuditLogService._internal();
  factory AuditLogService() => _instance;
  AuditLogService._internal();

  static const _retentionDays = 90;
  static const _logFileName = 'audit.log';
  String? _logDir;

  Future<void> _ensureLogDir() async {
    if (_logDir != null) return;
    final dir = await getApplicationDocumentsDirectory();
    _logDir = '${dir.path}/solosoul/audit';
    await Directory(_logDir!).create(recursive: true);
  }

  String _getLogFilePath() {
    final now = DateTime.now().toUtc();
    final dateStr = '${now.year}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
    return '$_logDir/audit_$dateStr.log';
  }

  /// 写入一条审计日志。
  Future<void> log(AuditLogEntry entry) async {
    await _ensureLogDir();
    final path = _getLogFilePath();
    final file = File(path);
    final line = '${entry.toJsonLine()}\n';
    await file.writeAsString(line, mode: FileMode.append, flush: true);
  }

  /// 快捷方法：记录字段敏感度变更。
  Future<void> logFieldSensitivityChanged({
    required String fieldKey,
    required String fieldLabel,
    required String oldSensitivity,
    required String newSensitivity,
    String? sectionName,
    String? accountId,
  }) async {
    await log(AuditLogEntry(
      event: AuditEventType.fieldSensitivityChanged,
      accountId: accountId,
      details: {
        'fieldKey': fieldKey,
        'fieldLabel': fieldLabel,
        'oldSensitivity': oldSensitivity,
        'newSensitivity': newSensitivity,
        if (sectionName != null) 'sectionName': sectionName,
      },
    ));
  }

  /// 快捷方法：记录字段语义类型变更。
  Future<void> logFieldSemanticTypeChanged({
    required String fieldKey,
    required String fieldLabel,
    required String? oldSemanticType,
    required String? newSemanticType,
    String? sectionName,
    String? accountId,
  }) async {
    await log(AuditLogEntry(
      event: AuditEventType.fieldSemanticTypeChanged,
      accountId: accountId,
      details: {
        'fieldKey': fieldKey,
        'fieldLabel': fieldLabel,
        'oldSemanticType': oldSemanticType,
        'newSemanticType': newSemanticType,
        if (sectionName != null) 'sectionName': sectionName,
      },
    ));
  }

  /// 快捷方法：记录插件安装授权。
  Future<void> logPluginInstallAuthorized({
    required String pluginId,
    required String pluginName,
    required List<Map<String, dynamic>> fieldDecisions,
    required String userStrategy,
    String? accountId,
  }) async {
    await log(AuditLogEntry(
      event: AuditEventType.pluginInstallAuthorized,
      accountId: accountId,
      details: {
        'pluginId': pluginId,
        'pluginName': pluginName,
        'fieldDecisions': fieldDecisions,
        'userStrategy': userStrategy,
      },
    ));
  }

  /// 快捷方法：记录运行时敏感度超出。
  Future<void> logPluginRuntimeSensitivityExceeded({
    required String pluginId,
    required String fieldKey,
    required String fieldLabel,
    required String actualSensitivity,
    required String requiredSensitivity,
    required String userStrategy,
    String? accountId,
  }) async {
    await log(AuditLogEntry(
      event: AuditEventType.pluginRuntimeSensitivityExceeded,
      accountId: accountId,
      details: {
        'pluginId': pluginId,
        'fieldKey': fieldKey,
        'fieldLabel': fieldLabel,
        'actualSensitivity': actualSensitivity,
        'requiredSensitivity': requiredSensitivity,
        'userStrategy': userStrategy,
      },
    ));
  }

  /// 快捷方法：记录插件字段访问被拒绝。
  Future<void> logPluginFieldAccessDenied({
    required String pluginId,
    required String fieldKey,
    required String fieldLabel,
    required String reason,
    String? accountId,
  }) async {
    await log(AuditLogEntry(
      event: AuditEventType.pluginFieldAccessDenied,
      accountId: accountId,
      details: {
        'pluginId': pluginId,
        'fieldKey': fieldKey,
        'fieldLabel': fieldLabel,
        'reason': reason,
      },
    ));
  }

  /// 快捷方法：记录插件字段访问掩码返回。
  Future<void> logPluginFieldAccessMasked({
    required String pluginId,
    required String fieldKey,
    required String fieldLabel,
    required String maskType,
    String? accountId,
  }) async {
    await log(AuditLogEntry(
      event: AuditEventType.pluginFieldAccessMasked,
      accountId: accountId,
      details: {
        'pluginId': pluginId,
        'fieldKey': fieldKey,
        'fieldLabel': fieldLabel,
        'maskType': maskType,
      },
    ));
  }

  /// 快捷方法：记录插件级字段映射变更。
  Future<void> logPluginFieldMappingChanged({
    required String pluginId,
    required String semanticType,
    required String? oldKey,
    required String? newKey,
    String? accountId,
  }) async {
    await log(AuditLogEntry(
      event: AuditEventType.pluginFieldMappingChanged,
      accountId: accountId,
      details: {
        'pluginId': pluginId,
        'semanticType': semanticType,
        'oldKey': oldKey,
        'newKey': newKey,
      },
    ));
  }

  /// 读取审计日志（最近 N 条）。
  Future<List<AuditLogEntry>> readRecent({int limit = 100}) async {
    await _ensureLogDir();
    final entries = <AuditLogEntry>[];

    // 获取按日期排序的日志文件（最新的在前）
    final dir = Directory(_logDir!);
    if (!await dir.exists()) return entries;

    final files = await dir
        .list()
        .where((e) => e is File && e.path.contains('audit_'))
        .cast<File>()
        .toList();
    files.sort((a, b) => b.path.compareTo(a.path));

    for (final file in files) {
      if (entries.length >= limit) break;
      final lines = await file.readAsLines();
      for (final line in lines.reversed) {
        if (line.trim().isEmpty) continue;
        try {
          final json = jsonDecode(line) as Map<String, dynamic>;
          entries.add(AuditLogEntry.fromJson(json));
          if (entries.length >= limit) break;
        } catch (_) {
          // 忽略损坏的行
        }
      }
    }

    return entries.reversed.toList();
  }

  /// 清理过期日志（保留 90 天）。
  Future<void> cleanupOldLogs() async {
    await _ensureLogDir();
    final cutoff = DateTime.now().toUtc().subtract(
      const Duration(days: _retentionDays),
    );

    final dir = Directory(_logDir!);
    if (!await dir.exists()) return;

    await for (final entity in dir.list()) {
      if (entity is File) {
        final stat = await entity.stat();
        if (stat.modified.isBefore(cutoff)) {
          await entity.delete();
        }
      }
    }
  }
}
