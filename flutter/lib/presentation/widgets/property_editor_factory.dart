import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

/// Factory that renders the correct editor or display for a PropertyValue.
class PropertyEditorFactory {
  static Widget? buildEditor({
    required PropertyValue property,
    required ValueChanged<PropertyValue> onChanged,
    bool readOnly = false,
  }) {
    return switch (property) {
      TextProperty(:final text, :final maxLength) => _TextEditor(
          text: text,
          maxLength: maxLength,
          onChanged: (v) => onChanged(TextProperty(text: v, maxLength: maxLength)),
          readOnly: readOnly,
        ),
      NumberProperty(:final value, :final decimalPlaces) => _NumberEditor(
          value: value,
          onChanged: (v) => onChanged(NumberProperty(value: v, decimalPlaces: decimalPlaces)),
          readOnly: readOnly,
        ),
      DateProperty(:final isoDate, :final includeTime) => _DateEditor(
          isoDate: isoDate,
          includeTime: includeTime,
          onChanged: (v) => onChanged(DateProperty(isoDate: v, includeTime: includeTime)),
          readOnly: readOnly,
        ),
      CheckboxProperty(:final checked) => _CheckboxEditor(
          checked: checked,
          onChanged: (v) => onChanged(CheckboxProperty(checked: v)),
          readOnly: readOnly,
        ),
      SelectProperty(:final options, :final selectedId) => _SelectEditor(
          options: options,
          selectedId: selectedId,
          onChanged: (v) => onChanged(SelectProperty(options: options, selectedId: v)),
          readOnly: readOnly,
        ),
      MultiSelectProperty(:final options, :final selectedIds) => _MultiSelectEditor(
          options: options,
          selectedIds: selectedIds,
          onChanged: (v) => onChanged(MultiSelectProperty(options: options, selectedIds: v)),
          readOnly: readOnly,
        ),
      RelationProperty(:final targetObjectId) => _RelationEditor(
          targetObjectId: targetObjectId,
          onChanged: (v) => onChanged(RelationProperty(targetObjectId: v)),
          readOnly: readOnly,
        ),
      UrlProperty(:final url) => _UrlEditor(
          url: url,
          onChanged: (v) => onChanged(UrlProperty(url: v)),
          readOnly: readOnly,
        ),
    };
  }

  static Widget buildDisplay(PropertyValue property) {
    return switch (property) {
      TextProperty(:final text) => Text(
          text.isEmpty ? '—' : text,
          style: text.isEmpty
              ? const TextStyle(fontStyle: FontStyle.italic, color: Colors.grey)
              : null,
        ),
      NumberProperty(:final value) => Text(
          value?.toString() ?? '—',
          style: value == null
              ? const TextStyle(fontStyle: FontStyle.italic, color: Colors.grey)
              : null,
        ),
      DateProperty(:final isoDate) => Text(
          isoDate?.isNotEmpty == true ? isoDate! : '—',
          style: isoDate == null || isoDate.isEmpty
              ? const TextStyle(fontStyle: FontStyle.italic, color: Colors.grey)
              : null,
        ),
      CheckboxProperty(:final checked) => Icon(
          checked ? Icons.check_box : Icons.check_box_outline_blank,
          color: checked ? Colors.green : Colors.grey,
        ),
      SelectProperty(:final options, :final selectedId) => Text(
          options.firstWhere((o) => o.id == selectedId,
              orElse: () => const SelectOption(id: '', label: '—', order: 0)).label,
        ),
      MultiSelectProperty(:final options, :final selectedIds) => () {
          if (selectedIds.isEmpty) {
            return const Text('—', style: TextStyle(fontStyle: FontStyle.italic, color: Colors.grey));
          }
          final labels = options
              .where((o) => selectedIds.contains(o.id))
              .map((o) => o.label)
              .join(', ');
          return Text(labels);
        }(),
      RelationProperty() => const Text('Relation'),
      UrlProperty(:final url) => Text(
          url ?? '—',
          style: url == null || url.isEmpty
              ? const TextStyle(fontStyle: FontStyle.italic, color: Colors.grey)
              : const TextStyle(color: Colors.blue),
        ),
    };
  }
}

// =============================================================================
// Internal Editors
// =============================================================================

class _TextEditor extends StatelessWidget {
  final String text;
  final int? maxLength;
  final ValueChanged<String> onChanged;
  final bool readOnly;

  const _TextEditor({
    required this.text,
    this.maxLength,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: TextEditingController(text: text)
        ..selection = TextSelection.collapsed(offset: text.length),
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
      ),
      maxLength: maxLength,
      readOnly: readOnly,
      onChanged: onChanged,
    );
  }
}

class _NumberEditor extends StatelessWidget {
  final double? value;
  final ValueChanged<double?> onChanged;
  final bool readOnly;

  const _NumberEditor({
    this.value,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: TextEditingController(text: value?.toString() ?? '')
        ..selection = TextSelection.collapsed(offset: value?.toString().length ?? 0),
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
      ),
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      readOnly: readOnly,
      onChanged: (v) {
        final parsed = double.tryParse(v);
        onChanged(parsed);
      },
    );
  }
}

class _DateEditor extends StatelessWidget {
  final String? isoDate;
  final bool includeTime;
  final ValueChanged<String?> onChanged;
  final bool readOnly;

  const _DateEditor({
    this.isoDate,
    this.includeTime = false,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: readOnly
          ? null
          : () async {
              final initial = DateTime.tryParse(isoDate ?? '');
              final picked = await showDatePicker(
                context: context,
                initialDate: initial ?? DateTime.now(),
                firstDate: DateTime(1900),
                lastDate: DateTime(2100),
              );
              if (picked != null) {
                onChanged(picked.toIso8601String().split('T').first);
              }
            },
      child: InputDecorator(
        decoration: const InputDecoration(
          border: OutlineInputBorder(),
        ),
        child: Text(
          isoDate ?? 'Select date',
          style: isoDate == null
              ? const TextStyle(color: Colors.grey)
              : null,
        ),
      ),
    );
  }
}

class _CheckboxEditor extends StatelessWidget {
  final bool checked;
  final ValueChanged<bool> onChanged;
  final bool readOnly;

  const _CheckboxEditor({
    required this.checked,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return CheckboxListTile(
      value: checked,
      onChanged: readOnly ? null : (v) => onChanged(v ?? false),
      controlAffinity: ListTileControlAffinity.leading,
      title: const Text('Enabled'),
      contentPadding: EdgeInsets.zero,
    );
  }
}

class _SelectEditor extends StatelessWidget {
  final List<SelectOption> options;
  final String? selectedId;
  final ValueChanged<String?> onChanged;
  final bool readOnly;

  const _SelectEditor({
    required this.options,
    this.selectedId,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return DropdownButtonFormField<String?>(
      value: selectedId,
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
      ),
      items: [
        const DropdownMenuItem(value: null, child: Text('—')),
        ...options.map((o) => DropdownMenuItem(
              value: o.id,
              child: Text(o.label),
            )),
      ],
      onChanged: readOnly ? null : onChanged,
    );
  }
}

class _MultiSelectEditor extends StatelessWidget {
  final List<SelectOption> options;
  final List<String> selectedIds;
  final ValueChanged<List<String>> onChanged;
  final bool readOnly;

  const _MultiSelectEditor({
    required this.options,
    required this.selectedIds,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 8,
      children: options.map((o) {
        final isSelected = selectedIds.contains(o.id);
        return FilterChip(
          label: Text(o.label),
          selected: isSelected,
          onSelected: readOnly
              ? null
              : (selected) {
                  final updated = List<String>.from(selectedIds);
                  if (selected) {
                    updated.add(o.id);
                  } else {
                    updated.remove(o.id);
                  }
                  onChanged(updated);
                },
        );
      }).toList(),
    );
  }
}

class _RelationEditor extends StatelessWidget {
  final String? targetObjectId;
  final ValueChanged<String?> onChanged;
  final bool readOnly;

  const _RelationEditor({
    this.targetObjectId,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    // MVP: relation picker is a simple text field for the target ID
    return TextField(
      controller: TextEditingController(text: targetObjectId ?? '')
        ..selection = TextSelection.collapsed(offset: targetObjectId?.length ?? 0),
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
        hintText: 'Target object ID',
      ),
      readOnly: readOnly,
      onChanged: (v) => onChanged(v.isEmpty ? null : v),
    );
  }
}

class _UrlEditor extends StatelessWidget {
  final String? url;
  final ValueChanged<String?> onChanged;
  final bool readOnly;

  const _UrlEditor({
    this.url,
    required this.onChanged,
    this.readOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: TextEditingController(text: url ?? '')
        ..selection = TextSelection.collapsed(offset: url?.length ?? 0),
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
        hintText: 'https://...',
      ),
      keyboardType: TextInputType.url,
      readOnly: readOnly,
      onChanged: (v) => onChanged(v.isEmpty ? null : v),
    );
  }
}
