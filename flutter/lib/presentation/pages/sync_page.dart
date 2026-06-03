import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/services/sync_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sync_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

class SyncPage extends ConsumerStatefulWidget {
  const SyncPage({super.key});

  @override
  ConsumerState<SyncPage> createState() => _SyncPageState();
}

class _SyncPageState extends ConsumerState<SyncPage> {
  final _pairingKeyController = TextEditingController();
  final _addressController = TextEditingController();
  final _responderKeyController = TextEditingController();
  bool _isPairingKeyVisible = false;
  bool _isResponderKeyVisible = false;

  @override
  void dispose() {
    _pairingKeyController.dispose();
    _addressController.dispose();
    _responderKeyController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final syncState = ref.watch(syncProvider);

    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(AppLocalizations.of(context).syncTitle),
        actions: [
          IconButton(
            icon: const Icon(Icons.article_outlined),
            tooltip: '同步日志',
            onPressed: () => _showSyncLogs(context),
          ),
          const HeaderActionButtons(),
        ],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Status Banner
            if (syncState.status == SyncStatus.syncing)
              _StatusBanner(
                icon: Icons.sync,
                message: l10n.syncSynchronizing,
                color: theme.colorScheme.primary,
              )
            else if (syncState.status == SyncStatus.success)
              _StatusBanner(
                icon: Icons.check_circle,
                message: switch (syncState.lastResult) {
                  null => l10n.syncComplete,
                  final result => _syncResultText(result),
                },
                color: Colors.green,
              )
            else if (syncState.status == SyncStatus.error)
              _StatusBanner(
                icon: Icons.error_outline,
                message: syncState.errorMessage ?? l10n.syncUnknownError,
                color: theme.colorScheme.error,
              ),

            const SizedBox(height: 16),

            // Device Discovery Section
            _SectionHeader(
              title: l10n.syncDeviceDiscovery,
              icon: Icons.radar,
            ),
            const SizedBox(height: 12),
            _DiscoveryCard(
              syncState: syncState,
              onDiscover: () => ref
                  .read(syncProvider.notifier)
                  .discoverDevices(timeoutMs: 5000),
              onDeviceTap: (device) => _showSyncDialog(device),
            ),

            const SizedBox(height: 24),

            // Manual Connection Section
            _SectionHeader(
              title: l10n.syncManualConnection,
              icon: Icons.link,
            ),
            const SizedBox(height: 12),
            _ManualConnectionCard(
              addressController: _addressController,
              pairingKeyController: _pairingKeyController,
              isPairingKeyVisible: _isPairingKeyVisible,
              onToggleVisibility: () =>
                  setState(() => _isPairingKeyVisible = !_isPairingKeyVisible),
              onConnect: _handleManualConnect,
              onCancel: () => ref.read(syncProvider.notifier).cancelSync(),
              isSyncing: syncState.status == SyncStatus.syncing,
            ),

            const SizedBox(height: 24),

            // Receive Sync Section
            _SectionHeader(
              title: l10n.syncReceiveSync,
              icon: Icons.download,
            ),
            const SizedBox(height: 12),
            _ReceiveSyncCard(
              pairingKeyController: _responderKeyController,
              isPairingKeyVisible: _isResponderKeyVisible,
              onToggleVisibility: () =>
                  setState(() => _isResponderKeyVisible = !_isResponderKeyVisible),
              onStartListening: _handleStartListening,
              onStopListening: _handleStopListening,
              isListening: syncState.isListening,
            ),

            const SizedBox(height: 24),

            // Pairing Key Section
            _SectionHeader(
              title: l10n.syncPairingKey,
              icon: Icons.vpn_key,
            ),
            const SizedBox(height: 12),
            _PairingKeyCard(
              onGenerateKey: _handleGenerateKey,
            ),

            const SizedBox(height: 24),

            // Last Sync Result
            if (syncState.lastResult case final result?) ...[
              _SectionHeader(
                title: l10n.syncLastSync,
                icon: Icons.history,
              ),
              const SizedBox(height: 12),
              _SyncResultCard(result: result),
            ],

            const SizedBox(height: 32),
          ],
        ),
      ),
    );
  }

  String _syncResultText(frb.SyncResult result) {
    final l10n = AppLocalizations.of(context);
    if (!result.success) return l10n.syncFailed(result.error ?? 'unknown error');
    return switch (result.direction) {
      frb.SyncDirection.pushed => l10n.syncDirectionPushed,
      frb.SyncDirection.pulled => l10n.syncDirectionPulled,
      frb.SyncDirection.merged => l10n.syncDirectionMerged,
      frb.SyncDirection.noChange => l10n.syncDirectionNoChange,
    };
  }

  void _showSyncDialog(frb.DiscoveredDevice device) {
    showDialog<void>(
      context: context,
      builder: (context) => _SyncDialog(
        device: device,
        onSync: (pairingKey) => _handleDeviceSync(device, pairingKey),
      ),
    );
  }

  Future<void> _handleDeviceSync(
    frb.DiscoveredDevice device,
    List<int> pairingKey,
  ) async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context).syncNoActiveAccount)),
        );
      }
      return;
    }

    final deviceSalt = await SyncService.instance.generateDeviceSalt();
    if (!mounted) return;
    await ref.read(syncProvider.notifier).syncWithDevice(
          accountId: accountId,
          device: device,
          pairingKey: pairingKey,
          deviceSalt: deviceSalt,
        );
  }

  Future<void> _handleManualConnect() async {
    final address = _addressController.text.trim();
    final pairingKeyHex = _pairingKeyController.text.trim();

    if (address.isEmpty || pairingKeyHex.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).syncEnterAddressAndKey)),
      );
      return;
    }

    final pairingKey = hexToBytes(pairingKeyHex);
    if (pairingKey == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).syncInvalidPairingKey)),
      );
      return;
    }

    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context).syncNoActiveAccount)),
        );
      }
      return;
    }

    final deviceSalt = await SyncService.instance.generateDeviceSalt();
    if (!mounted) return;
    await ref.read(syncProvider.notifier).syncWithAddress(
          accountId: accountId,
          remoteAddr: address,
          pairingKey: pairingKey,
          deviceSalt: deviceSalt,
        );
  }

  Future<void> _handleStartListening() async {
    final keyHex = _responderKeyController.text.trim();
    if (keyHex.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).syncEnterPairingKey)),
      );
      return;
    }
    final pairingKey = hexToBytes(keyHex);
    if (pairingKey == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).syncInvalidPairingKey)),
      );
      return;
    }
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context).syncNoActiveAccount)),
        );
      }
      return;
    }
    final deviceSalt = await SyncService.instance.generateDeviceSalt();
    if (!mounted) return;
    await ref.read(syncProvider.notifier).startListening(
          accountId: accountId,
          pairingKey: pairingKey,
          deviceSalt: deviceSalt,
        );
  }

  void _handleStopListening() {
    ref.read(syncProvider.notifier).stopListening();
  }

  Future<void> _handleGenerateKey() async {
    final key = await SyncService.instance.generatePairingKey();
    final hex = key.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
    await Clipboard.setData(ClipboardData(text: hex));
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).syncPairingKeyCopied)),
      );
    }
  }

  void _showSyncLogs(BuildContext context) {
    showDialog<void>(
      context: context,
      builder: (context) => const _SyncLogDialog(),
    );
  }
}

// =============================================================================
// Utilities
// =============================================================================

List<int>? hexToBytes(String hex) {
  hex = hex.replaceAll(RegExp(r'\s+'), '');
  if (hex.length % 2 != 0) return null;
  try {
    return List.generate(
      hex.length ~/ 2,
      (i) => int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16),
    );
  } on FormatException {
    return null;
  }
}

// =============================================================================
// Private Widgets
// =============================================================================

class _SectionHeader extends StatelessWidget {
  final String title;
  final IconData icon;

  const _SectionHeader({required this.title, required this.icon});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        Icon(icon, size: 20, color: theme.colorScheme.primary),
        const SizedBox(width: 8),
        Text(
          title,
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
      ],
    );
  }
}

class _StatusBanner extends StatelessWidget {
  final IconData icon;
  final String message;
  final Color color;

  const _StatusBanner({
    required this.icon,
    required this.message,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: color.withAlpha(25),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withAlpha(76)),
      ),
      child: Row(
        children: [
          Icon(icon, color: color, size: 20),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              message,
              style: TextStyle(color: color, fontWeight: FontWeight.w500),
            ),
          ),
        ],
      ),
    ).animate().fadeIn(duration: 300.ms);
  }
}

class _DiscoveryCard extends StatelessWidget {
  final SyncState syncState;
  final VoidCallback onDiscover;
  final void Function(frb.DiscoveredDevice) onDeviceTap;

  const _DiscoveryCard({
    required this.syncState,
    required this.onDiscover,
    required this.onDeviceTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDiscovering = syncState.status == SyncStatus.discovering;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    AppLocalizations.of(context).syncDiscoveryHint,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton.icon(
                  onPressed: isDiscovering ? null : onDiscover,
                  icon: isDiscovering
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.search, size: 18),
                  label: Text(isDiscovering ? AppLocalizations.of(context).syncScanning : AppLocalizations.of(context).syncScan),
                ),
              ],
            ),
            if (syncState.devices.isNotEmpty) ...[
              const SizedBox(height: 16),
              const Divider(),
              const SizedBox(height: 8),
              Text(
                AppLocalizations.of(context).syncFoundDevices(syncState.devices.length),
                style: theme.textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              ...syncState.devices.map(
                (device) => _DeviceTile(
                  device: device,
                  onTap: () => onDeviceTap(device),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _DeviceTile extends StatelessWidget {
  final frb.DiscoveredDevice device;
  final VoidCallback onTap;

  const _DeviceTile({required this.device, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListTile(
      leading: Icon(
        _deviceIcon(device.name),
        color: theme.colorScheme.primary,
      ),
      title: Text(device.name),
      subtitle: Text(
        device.addresses.isNotEmpty ? device.addresses.first : device.host,
        style: theme.textTheme.bodySmall,
      ),
      trailing: const Icon(Icons.sync, size: 20),
      onTap: onTap,
      contentPadding: EdgeInsets.zero,
    );
  }

  IconData _deviceIcon(String name) {
    final lower = name.toLowerCase();
    if (lower.contains('mac')) return Icons.laptop_mac;
    if (lower.contains('iphone') || lower.contains('ios')) {
      return Icons.phone_iphone;
    }
    if (lower.contains('android')) return Icons.phone_android;
    if (lower.contains('windows')) return Icons.desktop_windows;
    if (lower.contains('linux')) return Icons.computer;
    return Icons.devices;
  }
}

class _ManualConnectionCard extends StatelessWidget {
  final TextEditingController addressController;
  final TextEditingController pairingKeyController;
  final bool isPairingKeyVisible;
  final VoidCallback onToggleVisibility;
  final VoidCallback onConnect;
  final VoidCallback? onCancel;
  final bool isSyncing;

  const _ManualConnectionCard({
    required this.addressController,
    required this.pairingKeyController,
    required this.isPairingKeyVisible,
    required this.onToggleVisibility,
    required this.onConnect,
    this.onCancel,
    required this.isSyncing,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            TextField(
              controller: addressController,
              decoration: InputDecoration(
                labelText: AppLocalizations.of(context).syncRemoteAddress,
                hintText: AppLocalizations.of(context).syncRemoteAddressHint,
                prefixIcon: const Icon(Icons.computer),
                border: const OutlineInputBorder(),
              ),
              keyboardType: TextInputType.url,
            ),
            const SizedBox(height: 12),
            TextField(
              controller: pairingKeyController,
              decoration: InputDecoration(
                labelText: AppLocalizations.of(context).syncPairingKey,
                hintText: AppLocalizations.of(context).syncPairingKeyHint,
                prefixIcon: const Icon(Icons.vpn_key),
                border: const OutlineInputBorder(),
                suffixIcon: IconButton(
                  icon: Icon(
                    isPairingKeyVisible
                        ? Icons.visibility_off
                        : Icons.visibility,
                  ),
                  onPressed: onToggleVisibility,
                ),
              ),
              obscureText: !isPairingKeyVisible,
              maxLines: 1,
            ),
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: isSyncing
                  ? FilledButton.icon(
                      onPressed: onCancel,
                      style: FilledButton.styleFrom(
                        backgroundColor: theme.colorScheme.error,
                        foregroundColor: theme.colorScheme.onError,
                      ),
                      icon: const Icon(Icons.cancel, size: 18),
                      label: Text(AppLocalizations.of(context).commonCancel),
                    )
                  : FilledButton.icon(
                      onPressed: onConnect,
                      icon: const Icon(Icons.sync, size: 18),
                      label: Text(AppLocalizations.of(context).syncConnectSync),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ReceiveSyncCard extends StatelessWidget {
  final TextEditingController pairingKeyController;
  final bool isPairingKeyVisible;
  final VoidCallback onToggleVisibility;
  final VoidCallback onStartListening;
  final VoidCallback onStopListening;
  final bool isListening;

  const _ReceiveSyncCard({
    required this.pairingKeyController,
    required this.isPairingKeyVisible,
    required this.onToggleVisibility,
    required this.onStartListening,
    required this.onStopListening,
    required this.isListening,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.syncReceiveSyncHint,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: pairingKeyController,
              decoration: InputDecoration(
                labelText: l10n.syncPairingKey,
                hintText: l10n.syncPairingKeyHint,
                prefixIcon: const Icon(Icons.vpn_key),
                border: const OutlineInputBorder(),
                suffixIcon: IconButton(
                  icon: Icon(
                    isPairingKeyVisible ? Icons.visibility_off : Icons.visibility,
                  ),
                  onPressed: onToggleVisibility,
                ),
              ),
              obscureText: !isPairingKeyVisible,
              maxLines: 1,
            ),
            const SizedBox(height: 12),
            FutureBuilder<List<String>>(
              future: SyncService.getLocalIps(),
              builder: (context, snapshot) {
                final ips = snapshot.data ?? [];
                if (ips.isEmpty) return const SizedBox.shrink();
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (final ip in ips)
                        Text(
                          l10n.syncListeningAddress(ip, '9900'),
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.primary,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                    ],
                  ),
                );
              },
            ),
            Text(
              l10n.syncFirewallHint,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                onPressed: isListening ? onStopListening : onStartListening,
                icon: isListening
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.download, size: 18),
                label: Text(isListening ? l10n.syncStopListening : l10n.syncStartListening),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _PairingKeyCard extends StatelessWidget {
  final VoidCallback onGenerateKey;

  const _PairingKeyCard({required this.onGenerateKey});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              AppLocalizations.of(context).syncPairingKeyHint,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: onGenerateKey,
              icon: const Icon(Icons.key, size: 18),
              label: Text(AppLocalizations.of(context).syncGenerateAndCopyKey),
            ),
          ],
        ),
      ),
    );
  }
}

class _SyncResultCard extends StatelessWidget {
  final frb.SyncResult result;

  const _SyncResultCard({required this.result});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final error = result.error;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _ResultRow(
              label: l10n.syncStatus,
              value: result.success ? l10n.commonSuccess : l10n.syncTestFailed,
              icon: result.success ? Icons.check_circle : Icons.error_outline,
            ),
            const SizedBox(height: 8),
            _ResultRow(
              label: l10n.syncDirection,
              value: _directionText(context, result.direction),
              icon: _directionIcon(result.direction),
            ),
            const SizedBox(height: 8),
            _ResultRow(
              label: l10n.syncData,
              value: '${result.bytesSent} sent / ${result.bytesReceived} received',
              icon: Icons.swap_vert,
            ),
            const SizedBox(height: 8),
            _ResultRow(
              label: 'Attachments',
              value: '${result.attachmentsSent} sent / ${result.attachmentsReceived} received',
              icon: Icons.attach_file,
            ),
            if (result.attachmentIncomplete) ...[
              const SizedBox(height: 8),
              const _ResultRow(
                label: 'Warning',
                value: 'Some attachments were not fully transferred',
                icon: Icons.warning_amber,
              ),
            ],
            if (error != null && error.isNotEmpty) ...[
              const SizedBox(height: 8),
              _ResultRow(
                label: l10n.syncError,
                value: error,
                icon: Icons.error_outline,
              ),
            ],
          ],
        ),
      ),
    );
  }

  String _directionText(BuildContext context, frb.SyncDirection direction) {
    final l10n = AppLocalizations.of(context);
    return switch (direction) {
      frb.SyncDirection.pushed => l10n.syncDirectionPush,
      frb.SyncDirection.pulled => l10n.syncDirectionPull,
      frb.SyncDirection.merged => l10n.syncDirectionMergedShort,
      frb.SyncDirection.noChange => l10n.syncDirectionNoChangeShort,
    };
  }

  IconData _directionIcon(frb.SyncDirection direction) {
    return switch (direction) {
      frb.SyncDirection.pushed => Icons.upload,
      frb.SyncDirection.pulled => Icons.download,
      frb.SyncDirection.merged => Icons.merge,
      frb.SyncDirection.noChange => Icons.check_circle_outline,
    };
  }
}

class _ResultRow extends StatelessWidget {
  final String label;
  final String value;
  final IconData icon;

  const _ResultRow({
    required this.label,
    required this.value,
    required this.icon,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        Icon(icon, size: 16, color: theme.colorScheme.primary),
        const SizedBox(width: 8),
        Text(
          '$label: ',
          style: theme.textTheme.bodyMedium?.copyWith(
            fontWeight: FontWeight.w500,
          ),
        ),
        Expanded(
          child: Text(value, style: theme.textTheme.bodyMedium),
        ),
      ],
    );
  }
}

class _SyncDialog extends StatefulWidget {
  final frb.DiscoveredDevice device;
  final void Function(List<int> pairingKey) onSync;

  const _SyncDialog({required this.device, required this.onSync});

  @override
  State<_SyncDialog> createState() => _SyncDialogState();
}

class _SyncDialogState extends State<_SyncDialog> {
  final _keyController = TextEditingController();
  bool _isObscured = true;

  @override
  void dispose() {
    _keyController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(AppLocalizations.of(context).syncWithDevice(widget.device.name)),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            AppLocalizations.of(context).syncEnterPairingKey,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _keyController,
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).syncPairingKey,
              border: const OutlineInputBorder(),
              suffixIcon: IconButton(
                icon: Icon(
                  _isObscured ? Icons.visibility : Icons.visibility_off,
                ),
                onPressed: () => setState(() => _isObscured = !_isObscured),
              ),
            ),
            obscureText: _isObscured,
            autofocus: true,
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(AppLocalizations.of(context).commonCancel),
        ),
        FilledButton(
          onPressed: () {
            final hex = _keyController.text.trim();
            final bytes = hexToBytes(hex);
            if (bytes == null) {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text(AppLocalizations.of(context).syncInvalidPairingKey)),
              );
              return;
            }
            Navigator.pop(context);
            widget.onSync(bytes);
          },
          child: Text(AppLocalizations.of(context).syncButton),
        ),
      ],
    );
  }
}

class _SyncLogDialog extends ConsumerWidget {
  const _SyncLogDialog();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final syncState = ref.watch(syncProvider);
    final logs = syncState.syncLogs;
    final theme = Theme.of(context);

    return AlertDialog(
      title: Row(
        children: [
          const Icon(Icons.article_outlined, size: 20),
          const SizedBox(width: 8),
          const Expanded(child: Text('同步日志')),
          if (logs.isNotEmpty)
            Text(
              '${logs.length} 条',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
        ],
      ),
      content: SizedBox(
        width: double.maxFinite,
        height: 400,
        child: logs.isEmpty
            ? Center(
                child: Text(
                  '暂无同步日志',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              )
            : ListView.builder(
                itemCount: logs.length,
                itemBuilder: (context, index) {
                  final log = logs[index];
                  final isError = log.contains('[ERROR]');
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 2),
                    child: Text(
                      log,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: isError ? theme.colorScheme.error : null,
                        fontFamily: 'monospace',
                        fontFamilyFallback: const ['Menlo', 'Courier'],
                      ),
                    ),
                  );
                },
              ),
      ),
      actions: [
        TextButton(
          onPressed: logs.isEmpty
              ? null
              : () {
                  final text = logs.join('\n');
                  Clipboard.setData(ClipboardData(text: text));
                  Navigator.pop(context);
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('日志已复制到剪贴板')),
                  );
                },
          child: const Text('复制日志'),
        ),
        TextButton(
          onPressed: logs.isEmpty
              ? null
              : () => ref.read(syncProvider.notifier).clearSyncLogs(),
          child: const Text('清空日志'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('关闭'),
        ),
      ],
    );
  }
}
