import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/services/operation_log_aggregator.dart';
import 'package:solosoul_flutter/presentation/providers/services/profile_persistence_notifier.dart';

/// Service responsible for domain model updates (updateIdentity, updateTravel, etc.).
/// This class handles the updateXxx methods that:
/// - Log changes using OperationLogAggregator
/// - Save using ProfilePersistenceService
/// - Update account operation metadata
class SectionMutators {
  final Ref _ref;
  final OperationLogAggregator _logAggregator;
  final ProfilePersistenceService _persistence;

  SectionMutators(this._ref, this._logAggregator, this._persistence);

  /// Update identity data
  Future<bool> updateIdentity(IdentityData identity, ProfileData? current, void Function(ProfileData) updateState) async {
    final oldIdentity = current?.identity;

    _logAggregator.logIdentityChanges(oldIdentity, identity);

    final isCreate = oldIdentity == null;

    final newProfile = ProfileData(
      identity: identity,
      travel: current?.travel,
      financial: current?.financial,
      professional: current?.professional,
    );

    final result = await _persistence.saveProfile(newProfile);
    if (!result) return false;

    updateState(newProfile);

    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_logAggregator.summarizeIdentityChanges(oldIdentity, identity, isCreate));

    return true;
  }

  /// Add or update travel data
  Future<bool> updateTravel(TravelData travel, ProfileData? current, void Function(ProfileData) updateState) async {
    final oldTravel = current?.travel;

    _logAggregator.logTravelChanges(oldTravel, travel);

    final isCreate = oldTravel == null;

    final updated = (current ?? ProfileData()).copyWith(travel: travel);

    final result = await _persistence.saveProfile(updated);
    if (!result) return false;

    updateState(updated);

    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_logAggregator.summarizeTravelChanges(oldTravel, travel, isCreate));

    return result;
  }

  /// Update travel data with immediate save (bypasses debounce)
  Future<bool> updateTravelImmediate(TravelData travel, ProfileData? current, void Function(ProfileData) updateState) async {
    final oldTravel = current?.travel;

    _logAggregator.logTravelChanges(oldTravel, travel);
    final isCreate = oldTravel == null;

    final updated = (current ?? ProfileData()).copyWith(travel: travel);
    final result = await _persistence.saveProfileImmediate(updated);

    if (!result) return false;

    updateState(updated);
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_logAggregator.summarizeTravelChanges(oldTravel, travel, isCreate));

    return result;
  }

  /// Add or update financial data
  Future<bool> updateFinancial(FinancialData financial, ProfileData? current, void Function(ProfileData) updateState) async {
    final oldFinancial = current?.financial;

    _logAggregator.logFinancialChanges(oldFinancial, financial);

    final isCreate = oldFinancial == null;

    final updated = (current ?? ProfileData()).copyWith(financial: financial);

    final result = await _persistence.saveProfile(updated);
    if (!result) return false;

    updateState(updated);

    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_logAggregator.summarizeFinancialChanges(oldFinancial, financial, isCreate));

    return result;
  }

  /// Update financial data with immediate save (bypasses debounce)
  Future<bool> updateFinancialImmediate(FinancialData financial, ProfileData? current, void Function(ProfileData) updateState) async {
    final oldFinancial = current?.financial;

    _logAggregator.logFinancialChanges(oldFinancial, financial);
    final isCreate = oldFinancial == null;

    final updated = (current ?? ProfileData()).copyWith(financial: financial);
    final result = await _persistence.saveProfileImmediate(updated);

    if (!result) return false;

    updateState(updated);
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_logAggregator.summarizeFinancialChanges(oldFinancial, financial, isCreate));

    return result;
  }

  /// Add or update professional data
  Future<bool> updateProfessional(ProfessionalData professional, ProfileData? current, void Function(ProfileData) updateState) async {
    final oldProfessional = current?.professional;

    _logAggregator.logProfessionalChanges(oldProfessional, professional);

    final isCreate = oldProfessional == null;

    final updated = (current ?? ProfileData()).copyWith(professional: professional);

    final result = await _persistence.saveProfile(updated);
    if (!result) return false;

    updateState(updated);

    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_logAggregator.summarizeProfessionalChanges(oldProfessional, professional, isCreate));

    return result;
  }

  /// Update professional data with immediate save (bypasses debounce)
  Future<bool> updateProfessionalImmediate(ProfessionalData professional, ProfileData? current, void Function(ProfileData) updateState) async {
    final oldProfessional = current?.professional;

    _logAggregator.logProfessionalChanges(oldProfessional, professional);
    final isCreate = oldProfessional == null;

    final updated = (current ?? ProfileData()).copyWith(professional: professional);
    final result = await _persistence.saveProfileImmediate(updated);

    if (!result) return false;

    updateState(updated);
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_logAggregator.summarizeProfessionalChanges(oldProfessional, professional, isCreate));

    return result;
  }
}
