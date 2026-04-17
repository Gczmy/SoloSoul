import 'dart:convert';
import 'dart:io';
import 'package:path_provider/path_provider.dart';

/// Simple file-based secure storage for development/testing
/// This stores data in the app's Application Support directory
/// which is private to the app.
class SimpleSecureStorage {
  static SimpleSecureStorage? _instance;
  late Directory _storageDir;
  bool _initialized = false;

  SimpleSecureStorage._();

  static SimpleSecureStorage get instance {
    _instance ??= SimpleSecureStorage._();
    return _instance!;
  }

  Future<void> _ensureInitialized() async {
    if (_initialized) return;
    final appDir = await getApplicationSupportDirectory();
    _storageDir = Directory('${appDir.path}/solosoul_data');
    if (!await _storageDir.exists()) {
      await _storageDir.create(recursive: true);
    }
    _initialized = true;
  }

  String _getAccountsFile() => '${_storageDir.path}/accounts.json';
  String _getAccountFile(String accountId) => '${_storageDir.path}/account_$accountId.json';

  /// Read the accounts list
  Future<String?> _readAccounts() async {
    await _ensureInitialized();
    final file = File(_getAccountsFile());
    if (await file.exists()) {
      return await file.readAsString();
    }
    return null;
  }

  /// Write the accounts list
  Future<void> _writeAccounts(String data) async {
    await _ensureInitialized();
    final file = File(_getAccountsFile());
    await file.writeAsString(data);
  }

  /// List all accounts
  Future<Map<String, dynamic>> listAccounts() async {
    final data = await _readAccounts();
    if (data == null || data.isEmpty) {
      return {'accounts': []};
    }
    return jsonDecode(data) as Map<String, dynamic>;
  }

  /// Save accounts list
  Future<void> saveAccounts(List<Map<String, dynamic>> accounts) async {
    await _ensureInitialized();
    await _writeAccounts(jsonEncode({'accounts': accounts}));
  }

  /// Read account data
  Future<String?> readAccountData(String accountId) async {
    await _ensureInitialized();
    final file = File(_getAccountFile(accountId));
    if (await file.exists()) {
      return await file.readAsString();
    }
    return null;
  }

  /// Write account data
  Future<void> writeAccountData(String accountId, Map<String, dynamic> data) async {
    await _ensureInitialized();
    final file = File(_getAccountFile(accountId));
    await file.writeAsString(jsonEncode(data));
  }

  /// Delete account data
  Future<void> deleteAccountData(String accountId) async {
    await _ensureInitialized();
    final file = File(_getAccountFile(accountId));
    if (await file.exists()) {
      await file.delete();
    }
  }

  /// Update account salt and verification hash (for password change)
  Future<void> updateAccountSalt(String accountId, String newSalt, String newVerifyHash) async {
    await _ensureInitialized();
    final file = File(_getAccountFile(accountId));
    if (!await file.exists()) return;

    final content = await file.readAsString();
    final data = jsonDecode(content) as Map<String, dynamic>;
    data['salt'] = newSalt;
    data['verify_hash'] = newVerifyHash;
    await file.writeAsString(jsonEncode(data));
  }

  /// Update account metadata fields
  /// Supports: created_at, last_login_at, last_operation_at, last_operation_desc, recent_devices
  Future<void> updateAccountMetadata(
    String accountId, {
    DateTime? lastLoginAt,
    DateTime? lastOperationAt,
    String? lastOperationDesc,
    Map<String, dynamic>? device,
  }) async {
    await _ensureInitialized();
    final file = File(_getAccountFile(accountId));
    if (!await file.exists()) return;

    final content = await file.readAsString();
    final data = jsonDecode(content) as Map<String, dynamic>;

    if (lastLoginAt != null) {
      data['last_login_at'] = lastLoginAt.toIso8601String();
    }
    if (lastOperationAt != null) {
      data['last_operation_at'] = lastOperationAt.toIso8601String();
    }
    if (lastOperationDesc != null) {
      data['last_operation_desc'] = lastOperationDesc;
    }
    if (device != null) {
      final devices = (data['recent_devices'] as List<dynamic>?)
              ?.map((e) => e as Map<String, dynamic>)
              .toList() ??
          [];
      // Update existing device or add new
      final existingIdx = devices.indexWhere(
        (d) => d['device_name'] == device['device_name'],
      );
      if (existingIdx >= 0) {
        devices[existingIdx] = device;
      } else {
        // Keep only last 5 devices
        if (devices.length >= 5) {
          devices.removeAt(0);
        }
        devices.add(device);
      }
      data['recent_devices'] = devices;
    }

    await file.writeAsString(jsonEncode(data));
  }

  /// Set account creation time (only if not already set)
  Future<void> setAccountCreatedAt(String accountId, DateTime createdAt) async {
    await _ensureInitialized();
    final file = File(_getAccountFile(accountId));
    if (!await file.exists()) return;

    final content = await file.readAsString();
    final data = jsonDecode(content) as Map<String, dynamic>;

    // Only set if not already present (backward compatibility)
    if (!data.containsKey('created_at') || data['created_at'] == null) {
      data['created_at'] = createdAt.toIso8601String();
      await file.writeAsString(jsonEncode(data));
    }
  }
}
