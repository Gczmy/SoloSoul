// ============================================================
// SoloSoul Tauri — Shared TypeScript Type Definitions
//
// Minimal set of types that are actually imported across the
// frontend. Domain-specific types live next to their owners
// (ipc.ts, objectStore.ts, template.ts, etc.).
// ============================================================

/** Profile collection section identifiers */
export type ProfileSection = 'identity' | 'travel' | 'financial' | 'professional';

// ---- Theme System (per 09_ 4.3) ----

export type ThemePreset = 'warm-stone-light' | 'warm-stone-dark' | 'system';
export type AccentPreset = 'ocean' | 'amber' | 'forest' | 'rose' | 'purple' | 'custom';
export type BackgroundType = 'solid' | 'gradient' | 'image';

export interface ThemeConfig {
  preset: ThemePreset;
  accentColor: AccentPreset;
  customAccentHex?: string;
  backgroundType: BackgroundType;
  backgroundValue: string;
  defaultLightTheme?: string;
  defaultDarkTheme?: string;
}
