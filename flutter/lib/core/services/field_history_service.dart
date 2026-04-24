import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

/// Storage service for field histories.
/// Delegates to RustVaultService for encrypted persistence.
class FieldHistoryService {
  static FieldHistoryService? _instance;
  static FieldHistoryService get instance =>
      _instance ??= FieldHistoryService._();

  FieldHistoryService._();

  final RustVaultService _rustVault = RustVaultService.instance;

  /// Load histories for an account.
  Future<FormHistories> loadHistories(String accountId) async {
    final decrypted =
        await _rustVault.loadFieldHistoriesDecrypted(accountId);
    if (decrypted == null) {
      return FormHistories();
    }

    try {
      final json = jsonDecode(decrypted) as Map<String, dynamic>;
      return FormHistories.fromJson(json);
    } on Exception catch (_) {
      return FormHistories();
    }
  }

  /// Save histories for an account.
  Future<bool> saveHistories(String accountId, FormHistories histories) async {
    final jsonData = jsonEncode(histories.toJson());
    return await _rustVault.saveFieldHistoriesEncrypted(accountId, jsonData);
  }

  /// Add field history (single field mode).
  Future<FormHistories> addFieldHistory({
    required String accountId,
    required String itemId,
    required String fieldId,
    String? value,
    Map<String, String>? values,
    FormHistories? existingHistories,
  }) async {
    final histories = existingHistories ?? await loadHistories(accountId);

    FormHistories updated;
    if (values != null) {
      updated = histories.addSnapshot(itemId, fieldId, values);
    } else {
      updated = histories.addEntry(itemId, fieldId, value ?? '');
    }

    await saveHistories(accountId, updated);
    return updated;
  }

  /// Record a field change (single field mode).
  Future<FormHistories> recordFieldChange({
    required String accountId,
    required String itemId,
    required String fieldId,
    required String oldValue,
    FormHistories? existingHistories,
  }) async {
    if (oldValue.isEmpty) return existingHistories ?? FormHistories();
    return addFieldHistory(
      accountId: accountId,
      itemId: itemId,
      fieldId: fieldId,
      values: {fieldId: oldValue},
      existingHistories: existingHistories,
    );
  }

  /// Record a snapshot of all fields for an entry.
  Future<FormHistories> recordSnapshot({
    required String accountId,
    required String itemId,
    required String fieldIdPrefix,
    required Map<String, String> allFieldValues,
    FormHistories? existingHistories,
  }) async {
    return addFieldHistory(
      accountId: accountId,
      itemId: itemId,
      fieldId: fieldIdPrefix,
      values: allFieldValues,
      existingHistories: existingHistories,
    );
  }
}

/// Provider for field histories.
final fieldHistoriesProvider =
    NotifierProvider<FieldHistoriesNotifier, FormHistories>(() {
  return FieldHistoriesNotifier();
});

/// Notifier for managing field histories.
class FieldHistoriesNotifier extends Notifier<FormHistories> {
  String? _currentAccountId;

  @override
  FormHistories build() => FormHistories();

  FormHistories get histories => state;

  String? get _accountId =>
      ref.read(authNotifierProvider.notifier).selectedAccountId;

  /// Load histories for current account.
  Future<void> loadHistories([String? accountId]) async {
    final accId = accountId ?? _accountId;
    if (accId == null) return;
    // Skip if already loaded for this account with data
    if (_currentAccountId == accId && state.histories.isNotEmpty) {
      return;
    }
    _currentAccountId = accId;
    state = await FieldHistoryService.instance.loadHistories(accId);
  }

  /// Clear histories state (when auth is locked)
  void clearHistories() {
    _currentAccountId = null;
    state = FormHistories();
  }

  /// Reload histories from disk.
  Future<void> reloadHistories(String accountId) async {
    state = await FieldHistoryService.instance.loadHistories(accountId);
  }

  /// Record a field change.
  Future<void> recordFieldChange({
    required String accountId,
    required String itemId,
    required String fieldId,
    required String oldValue,
  }) async {
    if (oldValue.isEmpty) return;
    state = await FieldHistoryService.instance.recordFieldChange(
      accountId: accountId,
      itemId: itemId,
      fieldId: fieldId,
      oldValue: oldValue,
      existingHistories: state,
    );
  }

  /// Record a snapshot of all fields.
  Future<void> recordSnapshot({
    required String accountId,
    required String itemId,
    required String fieldIdPrefix,
    required Map<String, String> allFieldValues,
  }) async {
    state = await FieldHistoryService.instance.recordSnapshot(
      accountId: accountId,
      itemId: itemId,
      fieldIdPrefix: fieldIdPrefix,
      allFieldValues: allFieldValues,
      existingHistories: state,
    );
  }

  /// Get history for a specific field.
  FieldHistory? getHistory(String itemId, String fieldId) {
    return state.getHistory(itemId, fieldId);
  }

  /// Clear histories when account is locked.
  void clear() {
    state = FormHistories();
    _currentAccountId = null;
  }
}
