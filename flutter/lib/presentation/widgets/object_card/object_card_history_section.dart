import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart'
    show FieldHistory;
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';

class ObjectCardHistorySection extends StatelessWidget {
  final FieldHistory? history;

  const ObjectCardHistorySection({super.key, this.history});

  @override
  Widget build(BuildContext context) {
    return FieldHistoryView(
      fieldName: 'unified',
      history: history,
      initiallyExpanded: true,
    );
  }
}
