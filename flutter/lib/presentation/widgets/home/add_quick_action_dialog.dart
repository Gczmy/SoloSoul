import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/widgets/home/quick_action.dart';

class AddQuickActionDialog extends StatelessWidget {
  final List<QuickAction> actions;

  const AddQuickActionDialog({super.key, required this.actions});

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add Quick Action'),
      content: SizedBox(
        width: double.maxFinite,
        child: ListView.builder(
          shrinkWrap: true,
          itemCount: actions.length,
          itemBuilder: (context, index) {
            final action = actions[index];
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
              title: Text(action.label),
              onTap: () => Navigator.pop(context, action),
            );
          },
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
