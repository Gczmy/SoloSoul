import 'package:flutter/widgets.dart';
import '../../../src/renderer/liquid_glass_renderer.dart';

// =============================================================================
// kDefaultSheetSettings — shared glass preset for sheets
// =============================================================================

/// Default [LiquidGlassSettings] for both [GlassSheet] and [GlassModalSheet].
///
/// Centralised here so that all sheet types produce visually identical glass
/// following the Apple News / iOS 26 modal aesthetic:
/// - `thickness: 10` — moderate surface feel for large overlay surfaces.
/// - `blur: 10` — standard background frosting (matches iOS 26 overlay blur).
/// - `refractiveIndex: 0.15` — minimal rim. The lightweight shader computes
///   rim alpha as kRimAlphaBase(0.8) × refractiveIndex, so 1.2 produced an
///   opaque 0.96 rim — a visible hard "line" on large sheets. 0.15 gives
///   ~0.12 opacity — a barely-perceptible glassy edge matching iOS 26.
const kDefaultSheetSettings = LiquidGlassSettings(
  glassColor: Color(0x1FFFFFFF), // ~12% white — matches iOS 26 modal tint
  thickness: 10.0,
  blur: 10.0,
  lightIntensity: 0.7,
  lightAngle: 2.356194, // 0.75 * pi — upper-left, iOS 26 standard
  chromaticAberration: 0.0,
  refractiveIndex: 0.15, // Rim opacity = 0.8 × 0.15 = 0.12 (subtle edge)
  saturation: 1.2,
  ambientStrength: 0.4,
);
