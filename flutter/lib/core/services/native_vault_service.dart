import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart' show kDebugMode;

/// FFI bindings to Rust vault implementation (iOS/macOS only)
/// Uses JSON relay pattern to communicate with Rust vault store
class NativeVaultService {
  static NativeVaultService? _instance;
  late DynamicLibrary _lib;
  bool _isAndroid = false;

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

  void _initialize() {
    _isAndroid = Platform.isAndroid;

    _log('NativeVaultService initializing...');

    if (_isAndroid) {
      // Android: Vault operations not yet supported via FFI
      return;
    }

    // iOS/macOS: Load the native library
    if (Platform.isMacOS) {
      final exePath = File(Platform.resolvedExecutable).parent.path;
      final paths = [
        '$exePath/Frameworks/libsolosoul_core.dylib',
        '$exePath/../Frameworks/libsolosoul_core.dylib',
        '$exePath/../../Frameworks/libsolosoul_core.dylib',
        'libsolosoul_core.dylib',
        '../native/target/aarch64-apple-darwin/release/libsolosoul_core.dylib',
        '/Users/zzc/PycharmProjects/SoloSoul/flutter/native/target/release/libsolosoul_core.dylib',
        '/Users/zzc/PycharmProjects/SoloSoul/flutter/macos/Runner/Frameworks/libsolosoul_core.dylib',
      ];

      DynamicLibrary? loadedLib;
      for (final path in paths) {
        try {
          _lib = DynamicLibrary.open(path);
          loadedLib = _lib;
          _log('Successfully loaded dylib from: $path');
          break;
        } catch (e) {
          _log('Failed to load from $path: $e');
        }
      }

      if (loadedLib == null) {
        throw Exception('Failed to load libsolosoul_core.dylib');
      }
    } else if (Platform.isIOS) {
      _lib = DynamicLibrary.process();
    } else {
      throw UnsupportedError(
        'Unsupported platform: ${Platform.operatingSystem}',
      );
    }

    // Bind vault_request_ffi
    _vaultRequest = _lib
        .lookup<
          NativeFunction<
            Pointer<Utf8> Function(Pointer<Utf8> requestPtr, IntPtr requestLen)
          >
        >('vault_request_ffi')
        .asFunction();

    // Bind init_account_manager_ffi
    _initAccountManager = _lib
        .lookup<NativeFunction<Int32 Function(Pointer<Utf8> basePathPtr)>>(
          'init_account_manager_ffi',
        )
        .asFunction();

    // Bind is_vault_unlocked_ffi
    _isVaultUnlocked = _lib
        .lookup<NativeFunction<Int32 Function()>>('is_vault_unlocked_ffi')
        .asFunction();

    // Bind free_rust_string_ffi
    _freeRustString = _lib
        .lookup<NativeFunction<Void Function(Pointer<Utf8> ptr)>>(
          'free_rust_string_ffi',
        )
        .asFunction();
  }

  /// Initialize the account manager with base path
  bool initAccountManager(String basePath) {
    if (_isAndroid) {
      return false;
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
    if (_isAndroid) {
      return false;
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
    if (_isAndroid) {
      return null;
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
    final data = response!['data'] as Map<String, dynamic>;
    final dataB64 = data['data'] as String;
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
    // NOTE: _log() calls removed - they may cause hangs with synchronous file I/O
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
    final response = request(actionDeleteAccount, {'account_id': accountId});
    return _isSuccess(response);
  }

  /// List all accounts from Rust vault via JSON relay
  /// Returns list of account info maps with id, name, last_accessed
  List<Map<String, dynamic>>? listAccounts() {
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
}
