import 'package:flutter/material.dart';

/// 插件通用单选列表对话框
///
/// 不特定于任何插件，所有插件通过 `solosoul_show_dialog` 请求时均可复用。
class PluginRadioListDialog extends StatefulWidget {
  final String title;
  final String? description;
  final List<PluginRadioItem> items;

  const PluginRadioListDialog({
    super.key,
    required this.title,
    this.description,
    required this.items,
  });

  @override
  State<PluginRadioListDialog> createState() => _PluginRadioListDialogState();
}

class _PluginRadioListDialogState extends State<PluginRadioListDialog> {
  String? _selectedId;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      insetPadding: EdgeInsets.symmetric(
        horizontal: MediaQuery.of(context).size.width * 0.25,
        vertical: 24,
      ),
      title: Text(widget.title),
      content: SizedBox(
        width: double.maxFinite,
        child: Builder(
          builder: (context) {
            final desc = widget.description;
            return Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (desc != null && desc.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: Text(
                      desc,
                      style: TextStyle(
                        fontSize: 13,
                        color: Colors.grey.shade600,
                      ),
                    ),
                  ),
                Flexible(
                  child: RadioGroup<String>(
                    groupValue: _selectedId,
                    onChanged: (value) {
                      setState(() {
                        _selectedId = value;
                      });
                    },
                    child: ListView.builder(
                      shrinkWrap: true,
                      itemCount: widget.items.length,
                      itemBuilder: (context, index) {
                        final item = widget.items[index];
                        return RadioListTile<String>(
                          title: Text(item.label),
                          value: item.id,
                        );
                      },
                    ),
                  ),
                ),
          ],
        );
      },
    ),
  ),
  actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _selectedId == null
              ? null
              : () => Navigator.of(context).pop(_selectedId),
          child: const Text('确认'),
        ),
      ],
    );
  }
}

class PluginRadioItem {
  final String id;
  final String label;

  PluginRadioItem({required this.id, required this.label});
}
