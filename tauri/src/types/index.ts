// ============================================================
// SoloSoul Tauri — Shared TypeScript Type Definitions
//
// Single source of truth for frontend types that mirror the
// Rust backend (src-tauri/src/commands/*.rs).
// ============================================================

// ---------------------------------------------------------------------------
// Vault & Sensitivity
// ---------------------------------------------------------------------------

/** Vault lifecycle state as reported by the backend */
export type VaultState = 'uninitialized' | 'locked' | 'unlocked';

/**
 * Sensitivity levels for field-level classification.
 * Single source of truth: `core/sensitivity.rs` SensitivityLevel enum.
 */
export type SensitivityLevel = 'public' | 'internal' | 'sensitive' | 'critical';

/** Profile collection section identifiers */
export type ProfileSection = 'identity' | 'travel' | 'financial' | 'professional';

// ---------------------------------------------------------------------------
// Account
// ---------------------------------------------------------------------------

/** Account metadata returned by bootstrap / list_accounts */
export interface AccountInfo {
  id: string;
  name: string;
  salt: string;
  verifyHash: string;
  passwordHint?: string;
  createdAt?: string;
}

// ---------------------------------------------------------------------------
// Profile
// ---------------------------------------------------------------------------

/** Lightweight profile listing entry */
export interface ProfileSummary {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  version: number;
}

/** Full profile with binary payload */
export interface ProfileData {
  id: string;
  name: string;
  data: number[];
  createdAt: string;
  updatedAt: string;
  version: number;
}

/** Metadata for rendering a profile section card (HomePage grid) */
export interface ProfileSectionMeta {
  type: ProfileSection;
  label: string;
  icon: string;
  desc: string;
}

// ---------------------------------------------------------------------------
// Object (unified entity CRUD)
// ---------------------------------------------------------------------------

export interface ObjectSummary {
  id: string;
  name: string;
  collectionType: string;
  sectionType: string;
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
}

/** Full object payload (includes properties and optional soft-delete) */
export interface ObjectData {
  id: string;
  accountId: string;
  name: string;
  collectionType: string;
  properties: Record<string, unknown>;
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}

/** Command input: create a new object */
export interface CreateObjectInput {
  accountId: string;
  name: string;
  collectionType: string;
  properties: Record<string, unknown>;
}

/** Command input: update an existing object */
export interface UpdateObjectInput {
  name: string;
  properties: Record<string, unknown>;
  sensitivityLevel?: string;
}

/** Filter criteria for object listing */
export interface ObjectFilter {
  collectionType?: string;
  sensitivityLevel?: string;
  keyword?: string;
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/** A single search hit */
export interface SearchResultItem {
  objectId: string;
  name: string;
  collectionType: string;
  matchedField?: string;
  matchedValue?: string;
  relevance: number;
}

/** Paginated search response */
export interface SearchResult {
  items: SearchResultItem[];
  total: number;
  hasMore: boolean;
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/** App metadata returned by `get_app_info` command */
export interface AppInfo {
  appName: string;
  version: string;
  buildNumber: string;
  os: string;
  arch: string;
}

// ---------------------------------------------------------------------------
// API / IPC
// ---------------------------------------------------------------------------

/**
 * Generic response envelope for Tauri IPC commands.
 * Mirrors the repository pattern convention from `common/patterns.md`.
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  meta?: {
    total?: number;
    page?: number;
    limit?: number;
  };
}

// ---------------------------------------------------------------------------
// Common page prop types
// ---------------------------------------------------------------------------

/** Props shared by most page-level components */
export interface BasePageProps {
  onBack?: () => void;
  title?: string;
}

/** Props for components that require an unlocked account */
export interface WithAccountProps {
  accountId: string;
}

/** Standard async page status */
export type PageStatus = 'idle' | 'loading' | 'success' | 'error';

// ---- Theme System (per 09_ 4.3) ----

export type ThemePreset = 'warm-stone-light' | 'warm-stone-dark' | 'system';
export type AccentPreset = 'ocean' | 'amber' | 'forest' | 'rose' | 'custom';
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

export const ACCENT_COLORS: Record<AccentPreset, { light: string; dark: string }> = {
  ocean: { light: '#5B7C99', dark: '#7BA3C4' },
  amber: { light: '#C4925C', dark: '#D4A76A' },
  forest: { light: '#5B8C6F', dark: '#7AAF8F' },
  rose: { light: '#B06B7A', dark: '#D48A9A' },
  custom: { light: '#5B7C99', dark: '#7BA3C4' },
};
