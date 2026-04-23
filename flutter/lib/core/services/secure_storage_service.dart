import 'dart:convert';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Secure storage using flutter_secure_storage (Keychain on iOS/macOS,
/// EncryptedSharedPreferences on Android).
/// This replaces the previous JSON file-based storage with platform-native
/// secure storage that properly encrypts data at rest.
class SimpleSecureStorage {
  static SimpleSecureStorage? _instance;
  late final FlutterSecureStorage _secureStorage;
  bool _initialized = false;

  SimpleSecureStorage._();

  static SimpleSecureStorage get instance {
    _instance ??= SimpleSecureStorage._();
    return _instance!;
  }

  Future<void> _ensureInitialized() async {
    if (_initialized) return;
    // NOTE: flutter_secure_storage 9.x does not support after_first_unlock_this_device.
    // Using first_unlock_this_device instead - data is accessible after first device unlock
    // but NOT before. This is appropriate for passwords that should persist after initial
    // device setup while still protecting against cold-boot attacks.
    _secureStorage = const FlutterSecureStorage(
      aOptions: AndroidOptions(encryptedSharedPreferences: true),
      iOptions: IOSOptions(
        accessibility: KeychainAccessibility.first_unlock_this_device,
      ),
    );
    _initialized = true;
  }

  final String _accountsKey = 'solosoul_accounts';
  String _accountKey(String accountId) => 'solosoul_account_$accountId';

  /// Read the accounts list
  Future<String?> _readAccounts() async {
    await _ensureInitialized();
    return await _secureStorage.read(key: _accountsKey);
  }

  /// Write the accounts list
  Future<void> _writeAccounts(String data) async {
    await _ensureInitialized();
    await _secureStorage.write(key: _accountsKey, value: data);
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
    return await _secureStorage.read(key: _accountKey(accountId));
  }

  /// Write account data
  Future<void> writeAccountData(String accountId, Map<String, dynamic> data) async {
    await _ensureInitialized();
    await _secureStorage.write(key: _accountKey(accountId), value: jsonEncode(data));
  }

  /// Delete account data
  Future<void> deleteAccountData(String accountId) async {
    await _ensureInitialized();
    await _secureStorage.delete(key: _accountKey(accountId));
  }

  /// Update account salt and verification hash (for password change)
  Future<void> updateAccountSalt(String accountId, String newSalt, String newVerifyHash) async {
    await _ensureInitialized();
    final content = await _secureStorage.read(key: _accountKey(accountId));
    if (content == null) return;

    final data = jsonDecode(content) as Map<String, dynamic>;
    data['salt'] = newSalt;
    data['verify_hash'] = newVerifyHash;
    await _secureStorage.write(key: _accountKey(accountId), value: jsonEncode(data));
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
    final content = await _secureStorage.read(key: _accountKey(accountId));
    if (content == null) return;

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

    await _secureStorage.write(key: _accountKey(accountId), value: jsonEncode(data));
  }

  /// Set account creation time (only if not already set)
  Future<void> setAccountCreatedAt(String accountId, DateTime createdAt) async {
    await _ensureInitialized();
    final content = await _secureStorage.read(key: _accountKey(accountId));
    if (content == null) return;

    final data = jsonDecode(content) as Map<String, dynamic>;

    // Only set if not already present (backward compatibility)
    if (!data.containsKey('created_at') || data['created_at'] == null) {
      data['created_at'] = createdAt.toIso8601String();
      await _secureStorage.write(key: _accountKey(accountId), value: jsonEncode(data));
    }
  }
}
