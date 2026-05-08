import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/home/quick_action.dart';

class AddQuickActionDialog extends StatelessWidget {
  final List<QuickAction> actions;

  const AddQuickActionDialog({super.key, required this.actions});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final defaultActions = actions.where((a) => !a.isCustom).toList();
    final customActions = actions.where((a) => a.isCustom).toList();

    return AlertDialog(
      title: Text(l10n.dialogAddQuickAction),
      content: SizedBox(
        width: double.maxFinite,
        child: ListView(
          shrinkWrap: true,
          children: [
            if (defaultActions.isNotEmpty) ...[
              _SectionHeader(title: l10n.homeDefaultPages),
              ...defaultActions.map((action) => _ActionListTile(
                    action: action,
                    onTap: () => Navigator.pop(context, action),
                  ),),
            ],
            if (customActions.isNotEmpty) ...[
              if (defaultActions.isNotEmpty) const SizedBox(height: 16),
              _SectionHeader(title: l10n.homeCustomizedPages),
              ...customActions.map((action) => _ActionListTile(
                    action: action,
                    onTap: () => Navigator.pop(context, action),
                  ),),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l10n.commonCancel),
        ),
      ],
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;

  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(left: 16, top: 8, bottom: 4),
      child: Text(
        title,
        style: theme.textTheme.labelSmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
        ),
      ),
    );
  }
}

class _ActionListTile extends StatelessWidget {
  final QuickAction action;
  final VoidCallback onTap;

  const _ActionListTile({
    required this.action,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ListTile(
      leading: Container(
        width: 36,
        height: 36,
        decoration: BoxDecoration(
          color: action.color.withValues(alpha: 0.1),
          borderRadius: BorderRadius.circular(10),
        ),
        child: Icon(action.icon, color: action.color, size: 20),
      ),
      title: Text(action.localizedLabel(l10n)),
      onTap: onTap,
    );
  }
}
