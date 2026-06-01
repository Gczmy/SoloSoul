import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_actions_context.dart';

void main() {
  group('EntryActionsContext', () {
    testWidgets('provides callbacks to descendant widgets', (tester) async {
      VoidCallback? capturedEdit;
      VoidCallback? capturedDelete;

      await tester.pumpWidget(
        EntryActionsContext(
          onEdit: () => capturedEdit = () {},
          onDelete: () => capturedDelete = () {},
          child: Builder(
            builder: (context) {
              final actions = EntryActionsContext.of(context);
              // Trigger callbacks to verify they are passed through
              actions?.onEdit?.call();
              actions?.onDelete?.call();
              return const SizedBox.shrink();
            },
          ),
        ),
      );

      expect(capturedEdit, isNotNull);
      expect(capturedDelete, isNotNull);
    });

    testWidgets('returns null when no context in tree', (tester) async {
      await tester.pumpWidget(
        Builder(
          builder: (context) {
            final actions = EntryActionsContext.of(context);
            expect(actions, isNull);
            return const SizedBox.shrink();
          },
        ),
      );
    });

    testWidgets('updateShouldNotify returns true when callbacks change', (tester) async {
      var onEditCallCount = 0;
      final first = EntryActionsContext(
        onEdit: () => onEditCallCount++,
        child: const SizedBox.shrink(),
      );
      final second = EntryActionsContext(
        onEdit: () => onEditCallCount++,
        child: const SizedBox.shrink(),
      );
      // Different function instances => should notify
      expect(first.updateShouldNotify(second), isTrue);
    });

    testWidgets('updateShouldNotify returns false when callbacks are identical', (tester) async {
      void handler() {}
      final first = EntryActionsContext(
        onEdit: handler,
        onDelete: handler,
        child: const SizedBox.shrink(),
      );
      final second = EntryActionsContext(
        onEdit: handler,
        onDelete: handler,
        child: const SizedBox.shrink(),
      );
      expect(first.updateShouldNotify(second), isFalse);
    });

    testWidgets('passes onCopy and onToggleHistory callbacks', (tester) async {
      Future<void> Function(String)? capturedCopy;
      VoidCallback? capturedToggle;

      await tester.pumpWidget(
        EntryActionsContext(
          onCopy: (value) async { capturedCopy = (String s) async {}; },
          onToggleHistory: () { capturedToggle = () {}; },
          child: Builder(
            builder: (context) {
              final actions = EntryActionsContext.of(context);
              return GestureDetector(
                onTap: () {
                  actions?.onCopy?.call('test');
                  actions?.onToggleHistory?.call();
                },
              );
            },
          ),
        ),
      );

      final context = tester.element(find.byType(GestureDetector));
      final actions = EntryActionsContext.of(context);
      expect(actions?.onCopy, isNotNull);
      expect(actions?.onToggleHistory, isNotNull);
    });
  });
}
