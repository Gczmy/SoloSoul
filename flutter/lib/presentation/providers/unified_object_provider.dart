import 'dart:async';

import 'package:flutter/foundation.dart' show immutable;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/page_section_link_registry.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';

part 'unified_object_provider.g.dart';
part 'unified_object_notifier.dart';
part 'unified_object_cache.dart';
part 'unified_object_providers.dart';

/// Provider for unified object state management.
final unifiedObjectProvider =
    NotifierProvider<UnifiedObjectNotifier, UnifiedObjectData>(() {
  return UnifiedObjectNotifier();
});
