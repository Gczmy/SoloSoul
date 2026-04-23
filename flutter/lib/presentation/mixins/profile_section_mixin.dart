import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Base class that consolidates the repeated lifecycle + data-loading pattern
/// across profile section widgets.
///
/// Subclasses must:
/// - Extend [ProfileSectionState] instead of [ConsumerState]
/// - Override [loadItems] to load data from the notifier into local state
///
/// The base class handles WidgetsBindingObserver registration and dispatches
/// to [loadItems] on init and resume.
abstract class ProfileSectionState<T extends ConsumerStatefulWidget>
    extends ConsumerState<T> with WidgetsBindingObserver {
  /// Override to load items from the profile notifier.
  void loadItems();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    loadItems();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      loadItems();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }
}
