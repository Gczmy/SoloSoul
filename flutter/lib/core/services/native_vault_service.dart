import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:path_provider/path_provider.dart';
import 'package:pointycastle/export.dart';

import 'fallback_secure_storage.dart';
import 'native_crypto_service.dart';

/// FFI bindings to Rust vault implementation (iOS/macOS only)
/// Uses pure Dart implementation on Android and Windows with JSON file storage
class NativeVaultService {
  static NativeVaultService? _instance;
  late DynamicLibrary _lib;
  bool _isAndroid = false;
  bool _isWindows = false;

  // Android/Windows fallback storage
  FallbackSecureStorage? _fallbackSecureStorage;
  Directory? _profilesDir;
  bool _isUnlocked = false;
  String? _currentAccountId;

  /// Write debug log using dart:developer (debug build only)
  void _log(String msg) {
    if (!kDebugMode) return;
    developer.log(msg, name: 'NativeVault');
  }

  // FFI function types
  late Pointer<Utf8> Function(Pointer<Utf8> requestPtr, int requestLen)
      _vaultRequest;
  late int Function(Pointer<Utf8> basePathPtr) _initAccountManager;
  late int Function() _isVaultUnlocked;
  late void Function(Pointer<Utf8> ptr) _freeRustString;

  NativeVaultService._();

  static NativeVaultService get instance {
    _instance ??= NativeVaultService._().._initialize();
    return _instance!;
  }

  Future<void> _initializeAndroid() async {
    _fallbackSecureStorage = FallbackSecureStorage();
    final supportDir = await getApplicationSupportDirectory();
    _profilesDir = Directory('${supportDir.path}/solosoul_profiles');
    if (!await _profilesDir!.exists()) {
      await _profilesDir!.create(recursive: true);
    }
    _log('Android fallback vault initialized at: ${_profilesDir!.path}');
  }

  Future<void> _initializeWindows() async {
    _fallbackSecureStorage = FallbackSecureStorage();
    // Use %APPDATA% on Windows via path_provider
    final supportDir = await getApplicationSupportDirectory();
    _profilesDir = Directory('${supportDir.path}/solosoul_profiles');
    if (!await _profilesDir!.exists()) {
      await _profilesDir!.create(recursive: true);
    }
    _log('Windows fallback vault initialized at: ${_profilesDir!.path}');
  }

  void _initialize() {
    _isAndroid = Platform.isAndroid;
    _isWindows = Platform.isWindows;

    _log('NativeVaultService initializing...');

    if (_isAndroid) {
      // Android: Initialize fallback storage asynchronously
      _initializeAndroid();
      return;
    }

    if (_isWindows) {
      // Windows: Initialize fallback storage asynchronously
      _initializeWindows();
      return;
    }

    // iOS/macOS: Load the native library
    if (Platform.isMacOS || Platform.isIOS) {
      _lib = DynamicLibrary.process();
      _log('Loaded native library via DynamicLibrary.process()');
    } else {
      throw UnsupportedError(
        'Unsupported platform: ${Platform.operatingSystem}',
      );
    }

    // Bind vault_request_ffi
    _vaultRequest = _lib
        .lookup<
            NativeFunction<
                Pointer<Utf8> Function(Pointer<Utf8> requestPtr,
                    IntPtr requestLen)>>('vault_request_ffi')
        .asFunction();

    // Bind init_account_manager_ffi
    _initAccountManager = _lib
        .lookup<NativeFunction<Int32 Function(Pointer<Utf8> basePathPtr)>>(
            'init_account_manager_ffi')
        .asFunction();

    // Bind is_vault_unlocked_ffi
    _isVaultUnlocked = _lib
        .lookup<NativeFunction<Int32 Function()>>('is_vault_unlocked_ffi')
        .asFunction();

    // Bind free_rust_string_ffi
    _freeRustString = _lib
        .lookup<NativeFunction<Void Function(Pointer<Utf8> ptr)>>(
            'free_rust_string_ffi')
        .asFunction();
  }

  /// Initialize the account manager with base path
  bool initAccountManager(String basePath) {
    if (_isAndroid || _isWindows) {
      // Android/Windows fallback doesn't need explicit initialization
      return true;
    }

    final pathPtr = basePath.toNativeUtf8();
    try {
      final result = _initAccountManager(pathPtr);
      return result == 0;
    } finally {
      calloc.free(pathPtr);
    }
  }

  /// Check if vault is unlocked
  bool isVaultUnlocked() {
    if (_isAndroid || _isWindows) {
      return _isUnlocked;
    }

    final result = _isVaultUnlocked();
    return result == 1;
  }

  /// Vault request/response types
  static const String actionListProfiles = 'list_profiles';
  static const String actionSaveProfile = 'save_profile';
  static const String actionCreateProfile = 'create_profile';
  static const String actionLoadProfile = 'load_profile';
  static const String actionDeleteProfile = 'delete_profile';
  static const String actionGetVaultStats = 'get_vault_stats';
  static const String actionIsUnlocked = 'is_unlocked';
  static const String actionUnlockVault = 'unlock_vault';
  static const String actionLockVault = 'lock_vault';
  static const String actionCreateAccount = 'create_account';
  static const String actionDeleteAccount = 'delete_account';

  /// Make a vault request and return the response (public for cross-service use)
  Map<String, dynamic>? request(
    String action, [
    Map<String, dynamic>? payload,
  ]) {
    if (_isAndroid || _isWindows) {
      return _androidRequest(action, payload);
    }

    final request = {'action': action, if (payload != null) 'payload': payload};

    final requestJson = jsonEncode(request);
    final requestPtr = requestJson.toNativeUtf8();
    final requestLen = utf8.encode(requestJson).length;

    try {
      final responsePtr = _vaultRequest(requestPtr, requestLen);

      if (responsePtr == nullptr) {
        _log('responsePtr is nullptr for action: $action');
        return null;
      }

      final responseStr = responsePtr.toDartString();
      _freeRustString(responsePtr.cast());
      final response = jsonDecode(responseStr) as Map<String, dynamic>;
      _log('request completed: action=$action, success=${response['success']}');
      return response;
    } finally {
      calloc.free(requestPtr);
    }
  }

  /// Check if vault operation was successful
  bool _isSuccess(Map<String, dynamic>? response) {
    return response != null && response['success'] == true;
  }

  /// Get error message from response
  String? _getError(Map<String, dynamic>? response) {
    return response?['error'] as String?;
  }

  /// Test FFI connection with a simple ping
  bool ping() {
    final response = request('ping');
    return _isSuccess(response);
  }

  /// List all profiles
  List<Map<String, dynamic>>? listProfiles() {
    final response = request(actionListProfiles);
    if (!_isSuccess(response)) {
      return null;
    }
    final data = response!['data'] as List<dynamic>?;
    return data?.map((e) => e as Map<String, dynamic>).toList();
  }

  /// Save a profile (create or update)
  /// Returns profile summary on success
  Map<String, dynamic>? saveProfile(String name, Uint8List encryptedData) {
    final payload = {'name': name, 'data': base64Encode(encryptedData)};
    final response = request(actionSaveProfile, payload);
    if (!_isSuccess(response)) {
      return null;
    }
    return response!['data'] as Map<String, dynamic>?;
  }

  /// Create a new profile
  /// Returns profile summary on success
  Map<String, dynamic>? createProfile(String name, Uint8List encryptedData) {
    final payload = {'name': name, 'data': base64Encode(encryptedData)};
    final response = request(actionCreateProfile, payload);
    if (!_isSuccess(response)) {
      return null;
    }
    return response!['data'] as Map<String, dynamic>?;
  }

  /// Load a profile by ID
  /// Returns profile data (still encrypted) on success
  ({Uint8List data, Map<String, dynamic> summary})? loadProfile(String id) {
    final response = request(actionLoadProfile, {'id': id});
    if (!_isSuccess(response)) {
      return null;
    }
    final data = response!['data'] as Map<String, dynamic>?;
    if (data == null) return null;
    final dataB64 = data['data'] as String?;
    if (dataB64 == null || dataB64.isEmpty) return null;
    final encryptedData = base64Decode(dataB64);
    final summary = Map<String, dynamic>.from(data);
    summary.remove('data');
    return (data: Uint8List.fromList(encryptedData), summary: summary);
  }

  /// Delete a profile by ID
  bool deleteProfile(String id) {
    final response = request(actionDeleteProfile, {'id': id});
    return _isSuccess(response);
  }

  /// Get vault statistics
  Map<String, dynamic>? getVaultStats() {
    final response = request(actionGetVaultStats);
    if (!_isSuccess(response)) {
      return null;
    }
    return response!['data'] as Map<String, dynamic>?;
  }

  /// Check if vault is unlocked
  bool checkIsUnlocked() {
    final response = request(actionIsUnlocked);
    if (!_isSuccess(response)) {
      return false;
    }
    return response!['data']['is_unlocked'] == true;
  }

  /// Create a new account in the Rust vault
  /// Returns account info on success (including the generated account_id, salt, and verify_hash)
  ({
    bool success,
    String? error,
    String? accountId,
    String? name,
    String? salt,
    String? verifyHash,
  })?
  createAccount({required String name, required String password}) {
    if (_isAndroid || _isWindows) {
      // On Android/Windows, this must be called async - use createAccountAsync
      return (
        success: false,
        error: 'Use createAccountAsync on Android/Windows',
        accountId: null,
        name: null,
        salt: null,
        verifyHash: null,
      );
    }

    final payload = {
      'account_id': '', // Rust generates its own
      'name': name,
      'password': password,
    };
    final response = request(actionCreateAccount, payload);
    if (!_isSuccess(response)) {
      return (
        success: false,
        error: _getError(response),
        accountId: null,
        name: null,
        salt: null,
        verifyHash: null,
      );
    }
    final data = response!['data'] as Map<String, dynamic>;
    return (
      success: data['created'] == true,
      error: null,
      accountId: data['id'] as String?,
      name: data['name'] as String?,
      salt: data['salt'] as String?,
      verifyHash: data['verify_hash'] as String?,
    );
  }

  /// Unlock the vault with account_id and password
  /// Returns success status and crypto_version
  ({bool success, String? error, int? cryptoVersion})? unlockVault({
    required String accountId,
    required String password,
  }) {
    if (_isAndroid || _isWindows) {
      // On Android/Windows, this must be called async - use unlockVaultAsync
      return (success: false, error: 'Use unlockVaultAsync on Android/Windows', cryptoVersion: null);
    }

    final payload = {'account_id': accountId, 'password': password};
    final response = request(actionUnlockVault, payload);
    if (!_isSuccess(response)) {
      return (success: false, error: _getError(response), cryptoVersion: null);
    }
    final data = response!['data'] as Map<String, dynamic>;
    return (
      success: data['success'] == true,
      error: data['error'] as String?,
      cryptoVersion: data['crypto_version'] as int?,
    );
  }

  /// Lock the vault - clears session key and closes database connection
  void lockVault() {
    if (_isAndroid || _isWindows) {
      _androidLockVault();
      return;
    }
    request(actionLockVault);
  }

  /// Change account password
  /// Returns new salt and verify_hash on success
  ({bool success, String? error, String? salt, String? verifyHash})?
      changePassword({
    required String accountId,
    required String oldPassword,
    required String newPassword,
  }) {
    final payload = {
      'account_id': accountId,
      'old_password': oldPassword,
      'new_password': newPassword,
    };
    final response = request('change_password', payload);
    if (!_isSuccess(response)) {
      return (
        success: false,
        error: _getError(response),
        salt: null,
        verifyHash: null,
      );
    }
    final data = response!['data'] as Map<String, dynamic>;
    return (
      success: true,
      error: null,
      salt: data['salt'] as String?,
      verifyHash: data['verify_hash'] as String?,
    );
  }

  /// Get account config (salt and verify_hash) for Keychain migration
  ({
    String? id,
    String? name,
    String? salt,
    String? verifyHash,
    int? cryptoVersion,
  })?
  getAccountConfig({required String accountId}) {
    if (_isAndroid || _isWindows) {
      // On Android/Windows, this must be called async - use getAccountConfigAsync
      return null;
    }

    final payload = {'account_id': accountId};
    final response = request('get_account_config', payload);
    if (!_isSuccess(response)) {
      return null;
    }
    final data = response!['data'] as Map<String, dynamic>;
    return (
      id: data['id'] as String?,
      name: data['name'] as String?,
      salt: data['salt'] as String?,
      verifyHash: data['verify_hash'] as String?,
      cryptoVersion: data['crypto_version'] as int?,
    );
  }

  /// Delete an account and all its data from Rust vault
  bool deleteAccount({required String accountId}) {
    if (_isAndroid || _isWindows) {
      // On Android/Windows, this must be called async - use deleteAccountAsync
      return false;
    }

    final response = request(actionDeleteAccount, {'account_id': accountId});
    return _isSuccess(response);
  }

  /// List all accounts from Rust vault via JSON relay
  /// Returns list of account info maps with id, name, last_accessed
  List<Map<String, dynamic>>? listAccounts() {
    if (_isAndroid || _isWindows) {
      // On Android/Windows, this must be called async - use listAccountsAsync
      return null;
    }

    final response = request('list_accounts', <String, dynamic>{});
    if (!_isSuccess(response)) {
      return null;
    }
    final data = response!['data'] as Map<String, dynamic>?;
    if (data == null) {
      return null;
    }
    final accounts = data['accounts'] as List<dynamic>?;
    if (accounts == null) {
      return null;
    }
    return accounts.map((e) => Map<String, dynamic>.from(e as Map)).toList();
  }

  // ============================================================
  // Android/Windows Fallback Implementation (Async versions)
  // ============================================================

  /// Async version of createAccount for Android
  Future<({
    bool success,
    String? error,
    String? accountId,
    String? name,
    String? salt,
    String? verifyHash,
  })>
      createAccountAsync({required String name, required String password}) async {
    if (!_isUnlocked) {
      // Generate account ID
      final accountId = 'acc_${DateTime.now().millisecondsSinceEpoch}';

      // Generate salt
      final salt = NativeCryptoService.instance.generateSalt();
      if (salt == null) {
        return (
          success: false,
          error: 'Failed to generate salt',
          accountId: null,
          name: null,
          salt: null,
          verifyHash: null,
        );
      }

      // Derive key
      final derivedKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: salt,
      );
      if (derivedKey == null) {
        return (
          success: false,
          error: 'Failed to derive key',
          accountId: null,
          name: null,
          salt: null,
          verifyHash: null,
        );
      }

      // Generate verify_hash using SHA-256
      final verifyData = Uint8List.fromList([...derivedKey, ...salt]);
      final verifyHash = _sha256Hash(verifyData);

      // Store account metadata
      await _fallbackSecureStorage!.write(
        key: '${accountId}_name',
        value: name,
      );
      await _fallbackSecureStorage!.write(
        key: '${accountId}_salt',
        value: base64Encode(salt),
      );
      await _fallbackSecureStorage!.write(
        key: '${accountId}_verify_hash',
        value: base64Encode(verifyHash),
      );

      // Create verify test (encrypted known string)
      final verifyText = 'verify:$accountId';
      final verifyNonce = Uint8List.fromList(
        List.generate(12, (i) => (i * 17 + 43) % 256),
      );
      final verifyCiphertext = NativeCryptoService.instance.encrypt(
        data: Uint8List.fromList(utf8.encode(verifyText)),
        key: derivedKey,
        nonce: verifyNonce,
      );
      if (verifyCiphertext != null) {
        final verifyTest = Uint8List.fromList([...verifyNonce, ...verifyCiphertext]);
        await _fallbackSecureStorage!.write(
          key: '${accountId}_verify_test',
          value: base64Encode(verifyTest),
        );
      }

      // Track account ID
      final accountIdsStr = await _fallbackSecureStorage!.read(key: '__account_ids');
      List<String> accountIds = [];
      if (accountIdsStr != null) {
        accountIds = (jsonDecode(accountIdsStr) as List<dynamic>).cast<String>();
      }
      accountIds.add(accountId);
      await _fallbackSecureStorage!.write(
        key: '__account_ids',
        value: jsonEncode(accountIds),
      );

      return (
        success: true,
        error: null,
        accountId: accountId,
        name: name,
        salt: base64Encode(salt),
        verifyHash: base64Encode(verifyHash),
      );
    }
    return (
      success: false,
      error: 'Vault is already unlocked',
      accountId: null,
      name: null,
      salt: null,
      verifyHash: null,
    );
  }

  /// Async version of unlockVault for Android
  Future<({bool success, String? error, int? cryptoVersion})>
      unlockVaultAsync({
    required String accountId,
    required String password,
  }) async {
    // Retrieve stored salt and verify_hash for this account
    final saltStr =
        await _fallbackSecureStorage!.read(key: '${accountId}_salt');
    final verifyHashB64 =
        await _fallbackSecureStorage!.read(key: '${accountId}_verify_hash');

    if (saltStr == null || verifyHashB64 == null) {
      return (success: false, error: 'Account not found', cryptoVersion: null);
    }

    final salt = base64Decode(saltStr);

    // Derive key from password
    final derivedKey = NativeCryptoService.instance.deriveKey(
      password: password,
      salt: salt,
    );

    if (derivedKey == null) {
      return (success: false, error: 'Key derivation failed', cryptoVersion: null);
    }

    // Verify by trying to decrypt the verify test
    final verifyTestB64 = await _fallbackSecureStorage!.read(
      key: '${accountId}_verify_test',
    );

    if (verifyTestB64 != null) {
      final verifyTest = base64Decode(verifyTestB64);
      final nonce = Uint8List.fromList(verifyTest.sublist(0, 12));
      final ciphertext = Uint8List.fromList(verifyTest.sublist(12));
      final decrypted = NativeCryptoService.instance.decrypt(
        encrypted: ciphertext,
        key: derivedKey,
        nonce: nonce,
      );
      if (decrypted == null ||
          utf8.decode(decrypted) != 'verify:$accountId') {
        return (success: false, error: 'Invalid password', cryptoVersion: null);
      }
    }

    _currentAccountId = accountId;
    _isUnlocked = true;
    return (success: true, error: null, cryptoVersion: 1);
  }

  /// Async version of deleteAccount for Android
  Future<bool> deleteAccountAsync({required String accountId}) async {
    try {
      // Delete account files
      if (_profilesDir != null) {
        final files = _profilesDir!.listSync().whereType<File>().where(
              (f) => f.path.endsWith('.json'),
            );
        for (final file in files) {
          try {
            final content = file.readAsStringSync();
            final data = jsonDecode(content) as Map<String, dynamic>;
            if (data['account_id'] == accountId) {
              file.deleteSync();
            }
          } on Exception catch (_) {}
        }
      }

      // Delete secure storage keys
      await _fallbackSecureStorage!.delete(key: '${accountId}_name');
      await _fallbackSecureStorage!.delete(key: '${accountId}_salt');
      await _fallbackSecureStorage!.delete(key: '${accountId}_verify_hash');
      await _fallbackSecureStorage!.delete(key: '${accountId}_verify_test');

      // Remove from account IDs list
      final accountIdsStr =
          await _fallbackSecureStorage!.read(key: '__account_ids');
      if (accountIdsStr != null) {
        final accountIds =
            (jsonDecode(accountIdsStr) as List<dynamic>).cast<String>();
        accountIds.remove(accountId);
        await _fallbackSecureStorage!.write(
          key: '__account_ids',
          value: jsonEncode(accountIds),
        );
      }

      return true;
    } on Exception catch (e) {
      _log('deleteAccountAsync error: $e');
      return false;
    }
  }

  /// Async version of listAccounts for Android
  Future<List<Map<String, dynamic>>?> listAccountsAsync() async {
    try {
      final accounts = <Map<String, dynamic>>[];

      // List accounts by scanning secure storage keys
      final accountIdsStr =
          await _fallbackSecureStorage!.read(key: '__account_ids');
      if (accountIdsStr != null) {
        final accountIds =
            (jsonDecode(accountIdsStr) as List<dynamic>).cast<String>();
        for (final accId in accountIds) {
          final name =
              await _fallbackSecureStorage!.read(key: '${accId}_name');
          if (name != null) {
            accounts.add({
              'id': accId,
              'name': name,
              'last_accessed': DateTime.now().toIso8601String(),
            });
          }
        }
      }

      return accounts;
    } on Exception catch (e) {
      _log('listAccountsAsync error: $e');
      return null;
    }
  }

  /// Async version of getAccountConfig for Android
  Future<({
    String? id,
    String? name,
    String? salt,
    String? verifyHash,
    int? cryptoVersion,
  })?> getAccountConfigAsync({required String accountId}) async {
    try {
      final name =
          await _fallbackSecureStorage!.read(key: '${accountId}_name');
      final saltStr =
          await _fallbackSecureStorage!.read(key: '${accountId}_salt');
      final verifyHashB64 = await _fallbackSecureStorage!.read(
        key: '${accountId}_verify_hash',
      );

      if (name == null || saltStr == null || verifyHashB64 == null) {
        return null;
      }

      return (
        id: accountId,
        name: name,
        salt: saltStr,
        verifyHash: verifyHashB64,
        cryptoVersion: 1,
      );
    } on Exception catch (e) {
      _log('getAccountConfigAsync error: $e');
      return null;
    }
  }

  // ============================================================
  // Android/Windows Fallback Implementation (Sync versions for request())
  // ============================================================

  /// Handle vault requests on Android/Windows using JSON file storage
  Map<String, dynamic>? _androidRequest(
    String action,
    Map<String, dynamic>? payload,
  ) {
    switch (action) {
      case 'ping':
        return {'success': true, 'data': {'pong': true}};

      case actionListProfiles:
        return _androidListProfiles();

      case actionSaveProfile:
        return _androidSaveProfile(
          payload?['name'] as String,
          payload?['data'] as String?,
        );

      case actionCreateProfile:
        return _androidCreateProfile(
          payload?['name'] as String,
          payload?['data'] as String?,
        );

      case actionLoadProfile:
        return _androidLoadProfile(payload?['id'] as String);

      case actionDeleteProfile:
        return _androidDeleteProfile(payload?['id'] as String);

      case actionGetVaultStats:
        return _androidGetVaultStats();

      case actionIsUnlocked:
        return {'success': true, 'data': {'is_unlocked': _isUnlocked}};

      case actionUnlockVault:
        // unlockVault is async on Android/Windows - return error to prompt use of async version
        return {'success': false, 'error': 'Use unlockVaultAsync on Android/Windows'};

      case actionLockVault:
        _androidLockVault();
        return {'success': true};

      case 'change_password':
        return _androidChangePassword(
          payload?['account_id'] as String,
          payload?['old_password'] as String,
          payload?['new_password'] as String,
        );

      case actionCreateAccount:
        // createAccount is async on Android/Windows
        return {'success': false, 'error': 'Use createAccountAsync on Android/Windows'};

      case actionDeleteAccount:
        // deleteAccount is async on Android/Windows
        return {'success': false, 'error': 'Use deleteAccountAsync on Android/Windows'};

      case 'list_accounts':
        // listAccounts is async on Android/Windows
        return {'success': false, 'error': 'Use listAccountsAsync on Android/Windows'};

      case 'get_account_config':
        // getAccountConfig is async on Android/Windows
        return {'success': false, 'error': 'Use getAccountConfigAsync on Android/Windows'};

      default:
        _log('Unknown action on Android/Windows: $action');
        return {'success': false, 'error': 'Unknown action: $action'};
    }
  }

  Map<String, dynamic>? _androidListProfiles() {
    if (!_isUnlocked) {
      return {'success': false, 'error': 'Vault is locked'};
    }
    try {
      final files = _profilesDir!.listSync().whereType<File>().where(
            (f) => f.path.endsWith('.json'),
          );
      final profiles = <Map<String, dynamic>>[];
      for (final file in files) {
        try {
          final content = file.readAsStringSync();
          final data = jsonDecode(content) as Map<String, dynamic>;
          profiles.add({
            'id': data['id'],
            'name': data['name'],
            'created_at': data['created_at'],
            'updated_at': data['updated_at'],
          });
        } on Exception catch (_) {}
      }
      return {'success': true, 'data': profiles};
    } on Exception catch (e) {
      return {'success': false, 'error': 'Failed to list profiles: $e'};
    }
  }

  Map<String, dynamic>? _androidSaveProfile(String name, String? dataB64) {
    if (!_isUnlocked) {
      return {'success': false, 'error': 'Vault is locked'};
    }
    if (dataB64 == null) {
      return {'success': false, 'error': 'Missing name or data'};
    }
    try {
      final existing = _profilesDir!
          .listSync()
          .whereType<File>()
          .where((f) => f.path.endsWith('.json'))
          .where((f) {
            try {
              final content = f.readAsStringSync();
              final data = jsonDecode(content) as Map<String, dynamic>;
              return data['name'] == name;
            } on Exception catch (_) {
              return false;
            }
          }).toList();

      String id;
      String createdAt;
      if (existing.isNotEmpty) {
        final existingContent = existing.first.readAsStringSync();
        final existingData =
            jsonDecode(existingContent) as Map<String, dynamic>;
        id = existingData['id'] as String;
        createdAt = existingData['created_at'] as String;
      } else {
        id = DateTime.now().millisecondsSinceEpoch.toString();
        createdAt = DateTime.now().toIso8601String();
      }

      final profileData = {
        'id': id,
        'name': name,
        'data': dataB64,
        'created_at': createdAt,
        'updated_at': DateTime.now().toIso8601String(),
      };
      final file = File('${_profilesDir!.path}/$id.json');
      file.writeAsStringSync(jsonEncode(profileData));
      return {
        'success': true,
        'data': {
          'id': id,
          'name': name,
          'created_at': createdAt,
          'updated_at': profileData['updated_at'],
        },
      };
    } on Exception catch (e) {
      return {'success': false, 'error': 'Failed to save profile: $e'};
    }
  }

  Map<String, dynamic>? _androidCreateProfile(String name, String? dataB64) {
    // createProfile is same as saveProfile for Android
    return _androidSaveProfile(name, dataB64);
  }

  Map<String, dynamic>? _androidLoadProfile(String? id) {
    if (!_isUnlocked) {
      return {'success': false, 'error': 'Vault is locked'};
    }
    if (id == null) {
      return {'success': false, 'error': 'Missing profile id'};
    }
    try {
      final file = File('${_profilesDir!.path}/$id.json');
      if (!file.existsSync()) {
        return {'success': false, 'error': 'Profile not found'};
      }
      final content = file.readAsStringSync();
      final data = jsonDecode(content) as Map<String, dynamic>;
      return {
        'success': true,
        'data': {
          'id': data['id'],
          'name': data['name'],
          'data': data['data'],
          'created_at': data['created_at'],
          'updated_at': data['updated_at'],
        },
      };
    } on Exception catch (e) {
      return {'success': false, 'error': 'Failed to load profile: $e'};
    }
  }

  Map<String, dynamic>? _androidDeleteProfile(String? id) {
    if (!_isUnlocked) {
      return {'success': false, 'error': 'Vault is locked'};
    }
    if (id == null) {
      return {'success': false, 'error': 'Missing profile id'};
    }
    try {
      final file = File('${_profilesDir!.path}/$id.json');
      if (file.existsSync()) {
        file.deleteSync();
      }
      return {'success': true};
    } on Exception catch (e) {
      return {'success': false, 'error': 'Failed to delete profile: $e'};
    }
  }

  Map<String, dynamic>? _androidGetVaultStats() {
    if (!_isUnlocked) {
      return {'success': false, 'error': 'Vault is locked'};
    }
    try {
      final files = _profilesDir!.listSync().whereType<File>().where(
            (f) => f.path.endsWith('.json'),
          );
      return {
        'success': true,
        'data': {
          'profile_count': files.length,
          'account_id': _currentAccountId,
        },
      };
    } on Exception catch (e) {
      return {'success': false, 'error': 'Failed to get vault stats: $e'};
    }
  }

  void _androidLockVault() {
    _isUnlocked = false;
    _currentAccountId = null;
  }

  Map<String, dynamic>? _androidChangePassword(
    String? accountId,
    String? oldPassword,
    String? newPassword,
  ) {
    // For Android fallback, password change is not yet implemented
    return {
      'success': false,
      'error': 'Password change not supported on Android fallback'
    };
  }

  Uint8List _sha256Hash(Uint8List data) {
    // Use pointycastle for SHA-256 hash
    final digest = SHA256Digest();
    final hash = digest.process(data);
    return hash;
  }
}
