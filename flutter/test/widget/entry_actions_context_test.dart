import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_actions_context.dart';

void main() {
  group('EntryActionsContext', () {
    testWidgets('of returns context when found in tree', (tester) async {
      EntryActionsContext? found;

      await tester.pumpWidget(
        EntryActionsContext(
          onEdit: () {},
          onDelete: () {},
          child: Builder(
            builder: (context) {
              found = EntryActionsContext.of(context);
              return const SizedBox();
            },
          ),
        ),
      );

      expect(found, isNotNull);
      expect(found!.onEdit, isNotNull);
      expect(found!.onDelete, isNotNull);
    });

    testWidgets('of returns null when not in tree', (tester) async {
      EntryActionsContext? found;

      await tester.pumpWidget(
        Builder(
          builder: (context) {
            found = EntryActionsContext.of(context);
            return const SizedBox();
          },
        ),
      );

      expect(found, isNull);
    });

    testWidgets('updateShouldNotify returns true when callbacks change',
        (tester) async {
      var editCount = 0;

      await tester.pumpWidget(
        EntryActionsContext(
          onEdit: () => editCount++,
          child: const SizedBox(),
        ),
      );

      await tester.pumpWidget(
        EntryActionsContext(
          onEdit: () => editCount += 2,
          child: const SizedBox(),
        ),
      );

      // Different closures should trigger notify
      expect(editCount, 0);
    });

    testWidgets('updateShouldNotify returns false when callbacks are same',
        (tester) async {
      void onEdit() {}

      await tester.pumpWidget(
        EntryActionsContext(
          onEdit: onEdit,
          child: const SizedBox(),
        ),
      );

      await tester.pumpWidget(
        EntryActionsContext(
          onEdit: onEdit,
          child: const SizedBox(),
        ),
      );

      // Same closure should not trigger notify
      expect(tester.hasRunningAnimations, isFalse);
    });

    testWidgets('passes all callbacks to children', (tester) async {
      Future<void> onCopy(String s) async {}
      void onToggle() {}

      await tester.pumpWidget(
        EntryActionsContext(
          onEdit: () {},
          onDelete: () {},
          onCopy: onCopy,
          onToggleHistory: onToggle,
          child: Builder(
            builder: (context) {
              final ctx = EntryActionsContext.of(context)!;
              expect(ctx.onCopy, same(onCopy));
              expect(ctx.onToggleHistory, same(onToggle));
              return const SizedBox();
            },
          ),
        ),
      );
    });
  });
}
