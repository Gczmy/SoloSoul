export 'package:solosoul_flutter/presentation/providers/auth/auth_helpers.dart'
    show bytesToHex, hexToBytes, constantTimeEquals;

/// Thrown when password verification is blocked by brute-force protection.
class PasswordBackoffException implements Exception {
  final int remainingSeconds;
  final bool isLockedOut;

  const PasswordBackoffException({
    required this.remainingSeconds,
    required this.isLockedOut,
  });

  @override
  String toString() =>
      'PasswordBackoffException(remainingSeconds=$remainingSeconds, isLockedOut=$isLockedOut)';
}

/// Device info for tracking recent device logins
class DeviceInfo {
  final String deviceName;
  final DateTime lastUsed;
  const DeviceInfo({required this.deviceName, required this.lastUsed});

  factory DeviceInfo.fromJson(Map<String, dynamic> json) {
    return DeviceInfo(
      deviceName: json['device_name'] as String,
      lastUsed: DateTime.parse(json['last_used'] as String),
    );
  }

  Map<String, dynamic> toJson() => {
        'device_name': deviceName,
        'last_used': lastUsed.toIso8601String(),
      };
}

/// Account info
class AccountInfo {
  final String id;
  final String name;
  final String? passwordHint;
  final DateTime? lastAccessed;
  final DateTime? createdAt;
  final DateTime? lastLoginAt;
  final DateTime? lastOperationAt;
  final String? lastOperationDesc;
  final List<DeviceInfo> recentDevices;

  const AccountInfo({
    required this.id,
    required this.name,
    this.passwordHint,
    this.lastAccessed,
    this.createdAt,
    this.lastLoginAt,
    this.lastOperationAt,
    this.lastOperationDesc,
    this.recentDevices = const [],
  });

  factory AccountInfo.fromJson(Map<String, dynamic> json) {
    return AccountInfo(
      id: json['id'] as String,
      name: json['name'] as String,
      passwordHint: json['password_hint'] as String?,
      lastAccessed: json['last_accessed'] != null
          ? DateTime.parse(json['last_accessed'] as String)
          : null,
      createdAt: json['created_at'] != null
          ? DateTime.parse(json['created_at'] as String)
          : null,
      lastLoginAt: json['last_login_at'] != null
          ? DateTime.parse(json['last_login_at'] as String)
          : null,
      lastOperationAt: json['last_operation_at'] != null
          ? DateTime.parse(json['last_operation_at'] as String)
          : null,
      lastOperationDesc: json['last_operation_desc'] as String?,
      recentDevices: (json['recent_devices'] as List<dynamic>?)
              ?.map((e) => DeviceInfo.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'name': name,
        'password_hint': passwordHint,
        'last_accessed': lastAccessed?.toIso8601String(),
        'created_at': createdAt?.toIso8601String(),
        'last_login_at': lastLoginAt?.toIso8601String(),
        'last_operation_at': lastOperationAt?.toIso8601String(),
        'last_operation_desc': lastOperationDesc,
        'recent_devices': recentDevices.map((e) => e.toJson()).toList(),
      };

  AccountInfo copyWith({
    String? id,
    String? name,
    String? passwordHint,
    DateTime? lastAccessed,
    DateTime? createdAt,
    DateTime? lastLoginAt,
    DateTime? lastOperationAt,
    String? lastOperationDesc,
    List<DeviceInfo>? recentDevices,
  }) {
    return AccountInfo(
      id: id ?? this.id,
      name: name ?? this.name,
      passwordHint: passwordHint ?? this.passwordHint,
      lastAccessed: lastAccessed ?? this.lastAccessed,
      createdAt: createdAt ?? this.createdAt,
      lastLoginAt: lastLoginAt ?? this.lastLoginAt,
      lastOperationAt: lastOperationAt ?? this.lastOperationAt,
      lastOperationDesc: lastOperationDesc ?? this.lastOperationDesc,
      recentDevices: recentDevices ?? this.recentDevices,
    );
  }
}

/// Auth state
enum AuthState { initial, locked, unlocked, loading }
