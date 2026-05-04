import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

/// Generic page scaffold for object category pages (Profile, Travel, Financial, Professional).
///
/// Eliminates the boilerplate Scaffold -> AppBar -> SingleChildScrollView -> Column
/// duplication across category pages that only differ in their section content.
class ObjectCategoryPage extends ConsumerWidget {
  final String title;
  final List<Widget> sections;

  const ObjectCategoryPage({
    super.key,
    required this.title,
    required this.sections,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      appBar: AppBar(
        title: Text(title),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: sections,
        ),
      ),
    );
  }
}
