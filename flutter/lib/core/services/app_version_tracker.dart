// ===========================================================================
// AppVersionTracker — 检测应用版本变化，触发升级前备份
// ===========================================================================
// 在轻量存储中记录上次运行的 App 版本号。每次启动时对比当前版本：
// - 若版本变化，设置 [pendingUpgradeBackup] = true，调用方应在 Vault 解锁后
//   调用 [createUpgradeBackup] 触发一次保护性备份。
// - 首次运行（无记录）也视为变化。
//
// 存储使用 FallbackSecureStorage（Keychain → 文件回退），不依赖 Vault 解锁。
// ===========================================================================

import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';

class AppVersionTracker {
  static const _key = 'solosoul_last_app_version';
  static AppVersionTracker? _instance;

  final FallbackSecureStorage _storage;
  String? _currentVersion;
  bool _pendingUpgradeBackup = false;

  AppVersionTracker._() : _storage = FallbackSecureStorage();

  static AppVersionTracker get instance {
    _instance ??= AppVersionTracker._();
    return _instance!;
  }

  /// 当前检测到的 App 版本（仅在 [checkVersion] 成功后有效）。
  String? get currentVersion => _currentVersion;

  /// 是否需要执行升级保护备份。
  bool get pendingUpgradeBackup => _pendingUpgradeBackup;

  /// 检测版本是否变化。应在应用启动时调用一次。
  Future<void> checkVersion(String currentVersion) async {
    _currentVersion = currentVersion;
    final previous = await _storage.read(key: _key);
    if (previous == null || previous != currentVersion) {
      _pendingUpgradeBackup = true;
      await _storage.write(key: _key, value: currentVersion);
    } else {
      _pendingUpgradeBackup = false;
    }
  }

  /// 清除待处理标记（备份成功后调用）。
  void clearPendingBackup() {
    _pendingUpgradeBackup = false;
  }
}
