// ignore_for_file: public_member_api_docs

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:meta/meta.dart';

import '_env_web.dart' if (dart.library.io) '_env_io.dart';

const String _shadersRoot =
    !kIsWeb && isTestEnvironment ? '' : 'packages/liquid_glass_widgets/';

@internal
abstract class ShaderKeys {
  const ShaderKeys._();

  static const blendedGeometry =
      '${_shadersRoot}shaders/liquid_glass_geometry_blended.frag';

  static const liquidGlassRender =
      '${_shadersRoot}shaders/liquid_glass_final_render.frag';
}
