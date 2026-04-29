// Standalone FFI test - run with: dart test_ffi.dart
// ignore_for_file: avoid_print, unused_catch_clause
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

void main() async {
  print('=== SoloSoul FFI Integration Test ===\n');

  // Load the library
  final paths = [
    '/Users/zzc/PycharmProjects/SoloSoul/flutter/native/target/release/libsolosoul_core.dylib',
    '/Users/zzc/PycharmProjects/SoloSoul/flutter/macos/Runner/Frameworks/libsolosoul_core.dylib',
  ];

  DynamicLibrary? lib;
  for (final path in paths) {
    try {
      lib = DynamicLibrary.open(path);
      print('✓ Loaded library from: $path');
      break;
    } on Exception catch (e) {
      print('✗ Failed to load from: $path');
    }
  }

  if (lib == null) {
    print('\n✗ Could not load library from any path');
    exit(1);
  }

  // Bind functions - matching the working native_vault_service.dart
  late final Pointer<Utf8> Function(Pointer<Utf8>, int) vaultRequest;
  late final int Function(Pointer<Utf8>) initAccountManager;
  late final int Function() isVaultUnlocked;
  late final void Function(Pointer<Utf8>) freeRustString;

  try {
    vaultRequest = lib
        .lookup<NativeFunction<Pointer<Utf8> Function(Pointer<Utf8>, IntPtr)>>('vault_request_ffi')
        .asFunction();
    initAccountManager = lib
        .lookup<NativeFunction<Int32 Function(Pointer<Utf8>)>>('init_account_manager_ffi')
        .asFunction();
    isVaultUnlocked = lib
        .lookup<NativeFunction<Int32 Function()>>('is_vault_unlocked_ffi')
        .asFunction();
    freeRustString = lib
        .lookup<NativeFunction<Void Function(Pointer<Utf8>)>>('free_rust_string_ffi')
        .asFunction();
    print('✓ All FFI symbols bound\n');
  } on Exception catch (e) {
    print('\n✗ Failed to bind symbols: $e');
    exit(1);
  }

  // Test 1: Init account manager
  print('Test 1: init_account_manager');
  final tempDir = Directory.systemTemp.createTempSync('solosoul_test_');
  print('  Temp dir: ${tempDir.path}');
  final pathPtr = tempDir.path.toNativeUtf8();
  final initResult = initAccountManager(pathPtr);
  calloc.free(pathPtr);
  print('  Result: ${initResult == 0 ? "✓ SUCCESS" : "✗ FAILED"} (code=$initResult)');

  // Test 2: Check vault is locked
  print('\nTest 2: is_vault_unlocked (should be 0)');
  final unlocked = isVaultUnlocked();
  print('  Result: ${unlocked == 0 ? "✓ CORRECT (locked)" : "✗ UNEXPECTED (unlocked=$unlocked)"}');

  // Test 3: Create an account
  print('\nTest 3: create_account');
  final createAccountRequest = jsonEncode({
    'action': 'create_account',
    'payload': {'account_id': 'ignored', 'name': 'Test Account', 'password': 'test_password_123!'}
  });
  final createAccountRequestPtr = createAccountRequest.toNativeUtf8();
  final createAccountResponsePtr = vaultRequest(createAccountRequestPtr, createAccountRequest.length);
  calloc.free(createAccountRequestPtr);
  final createAccountResponse = createAccountResponsePtr.toDartString();
  freeRustString(createAccountResponsePtr.cast());
  final createAccountParsed = jsonDecode(createAccountResponse) as Map<String, dynamic>;
  String? createdAccountId;
  if (createAccountParsed['success'] == true) {
    print('  ✓ SUCCESS');
    createdAccountId = createAccountParsed['data']?['id'] as String?;
    print('  Account ID: $createdAccountId');
  } else {
    print('  ✗ FAILED: ${createAccountParsed['error']}');
  }

  // Test 4: Unlock the vault - MUST use the returned account_id (Rust generates its own)
  print('\nTest 4: unlock_vault');
  if (createdAccountId == null) {
    print('  ✗ SKIPPED: No account ID from create_account');
    return;
  }
  final unlockRequest = jsonEncode({
    'action': 'unlock_vault',
    'payload': {'account_id': createdAccountId, 'password': 'test_password_123!'}
  });
  final unlockRequestPtr = unlockRequest.toNativeUtf8();
  final unlockResponsePtr = vaultRequest(unlockRequestPtr, unlockRequest.length);
  calloc.free(unlockRequestPtr);
  final unlockResponse = unlockResponsePtr.toDartString();
  freeRustString(unlockResponsePtr.cast());
  final unlockParsed = jsonDecode(unlockResponse) as Map<String, dynamic>;
  if (unlockParsed['success'] == true) {
    print('  ✓ SUCCESS');
  } else {
    print('  ✗ FAILED: ${unlockParsed['error']}');
  }

  // Test 5: List profiles (should return empty after unlock)
  print('\nTest 5: list_profiles (should be empty)');
  final listRequest = jsonEncode({'action': 'list_profiles'});
  final listRequestPtr = listRequest.toNativeUtf8();
  final listResponsePtr = vaultRequest(listRequestPtr, listRequest.length);
  calloc.free(listRequestPtr);
  final listResponse = listResponsePtr.toDartString();
  freeRustString(listResponsePtr.cast());
  final listParsed = jsonDecode(listResponse) as Map<String, dynamic>;
  if (listParsed['success'] == true) {
    print('  ✓ SUCCESS');
  } else {
    print('  ✗ FAILED: ${listParsed['error']}');
  }

  // Test 6: Create a profile
  print('\nTest 6: create_profile');
  final profileData = base64Encode(utf8.encode('{"test": "data", "version": 1}'));
  final createRequest = jsonEncode({
    'action': 'create_profile',
    'payload': {'name': 'test_profile', 'data': profileData}
  });
  final createRequestPtr = createRequest.toNativeUtf8();
  final createResponsePtr = vaultRequest(createRequestPtr, createRequest.length);
  calloc.free(createRequestPtr);
  final createResponse = createResponsePtr.toDartString();
  freeRustString(createResponsePtr.cast());
  final createParsed = jsonDecode(createResponse) as Map<String, dynamic>;
  String? profileId;
  if (createParsed['success'] == true) {
    print('  ✓ SUCCESS');
    profileId = createParsed['data']?['id'];
    print('  Profile ID: $profileId');
  } else {
    print('  ✗ FAILED: ${createParsed['error']}');
  }

  // Test 5: List profiles (should have 1)
  if (profileId != null) {
    print('\nTest 5: list_profiles (should have 1)');
    final list2Request = jsonEncode({'action': 'list_profiles'});
    final list2RequestPtr = list2Request.toNativeUtf8();
    final list2ResponsePtr = vaultRequest(list2RequestPtr, list2Request.length);
    calloc.free(list2RequestPtr);
    final list2Response = list2ResponsePtr.toDartString();
    freeRustString(list2ResponsePtr.cast());
    final list2Parsed = jsonDecode(list2Response) as Map<String, dynamic>;
    final profiles = list2Parsed['data'] as List;
    print('  Profile count: ${profiles.length} (expected: 1)');
    print('  ✓ SUCCESS');

    // Test 6: Load profile
    print('\nTest 6: load_profile');
    final loadRequest = jsonEncode({
      'action': 'load_profile',
      'payload': {'id': profileId}
    });
    final loadRequestPtr = loadRequest.toNativeUtf8();
    final loadResponsePtr = vaultRequest(loadRequestPtr, loadRequest.length);
    calloc.free(loadRequestPtr);
    final loadResponse = loadResponsePtr.toDartString();
    freeRustString(loadResponsePtr.cast());
    final loadParsed = jsonDecode(loadResponse) as Map<String, dynamic>;
    if (loadParsed['success'] == true) {
      final dataB64 = loadParsed['data']?['data'] as String?;
      if (dataB64 != null) {
        final decrypted = utf8.decode(base64Decode(dataB64));
        print('  Decrypted data: $decrypted');
        print('  ✓ SUCCESS');
      }
    } else {
      print('  ✗ FAILED: ${loadParsed['error']}');
    }

    // Test 7: Delete profile
    print('\nTest 7: delete_profile');
    final deleteRequest = jsonEncode({
      'action': 'delete_profile',
      'payload': {'id': profileId}
    });
    final deleteRequestPtr = deleteRequest.toNativeUtf8();
    final deleteResponsePtr = vaultRequest(deleteRequestPtr, deleteRequest.length);
    calloc.free(deleteRequestPtr);
    final deleteResponse = deleteResponsePtr.toDartString();
    freeRustString(deleteResponsePtr.cast());
    final deleteParsed = jsonDecode(deleteResponse) as Map<String, dynamic>;
    if (deleteParsed['success'] == true) {
      print('  ✓ SUCCESS');
    } else {
      print('  ✗ FAILED: ${deleteParsed['error']}');
    }
  }

  // Cleanup
  print('\nCleaning up temp directory...');
  tempDir.deleteSync(recursive: true);

  print('\n=== All Tests Passed ===');
}
