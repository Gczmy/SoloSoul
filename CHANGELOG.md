# Changelog

All notable changes to SoloSoul are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.6.1] - 2026-05-09

### Added

- **Trash Filter Section** — Filter by time (10 days/1 day/6 hours/1 hour) and type (Page/Section/Item)
- **Trash Hierarchy** — Expand pages to see sections, expand sections to see items

### Fixed

- **Delete Dialog i18n** — Localized delete confirmation dialogs
- **Item Type Filter** — Fixed "Item" filter matching predefined section items

## [1.6.0] - 2026-05-09

### Added

- **Custom Sections on Default Pages** — Add custom sections via "+" button on Profile/Travel/Financial/Professional pages, reused from the custom page section system
- **OCR Scan: Save Original File** — Scanned images can now be saved as encrypted attachments in the vault
- **MRZ Scan Section Selector** — Manual override for import target section when scanning MRZ documents
- **Operation Log i18n** — Action labels (Create/Update/Delete/Restore/Purge), time labels (Just now/Xm ago/Xh ago/Xd ago), device platform labels, and section names all localized to Chinese

### Fixed

- **MRZ Visa Routing** — Fixed double-save guard for MRZ scan imports to prevent duplicate entries
- **Section Deletion Notification** — Fixed snackbar disappearing after section soft-delete by capturing overlay before async gap
- **Trash Children Sorting** — Deleted children now sorted by deletion time (newest first)
- **Custom Section Title i18n** — Property key 'Title' now correctly resolves to localized label
- **Trash Detail Dialog** — Empty properties no longer show "(empty)" text, reducing visual noise for sections

### i18n

- **Display Card Labels** — All property labels on Profile/Travel/Financial/Professional card views are now localized using `translateFieldLabel`
- **Operation Log** — Filter labels, action badges, time stamps, device tags, and section labels fully localized

## [1.5.1] - 2026-05-08

### Fixed

- **Password Hint Persistence** — Account creation now saves hint to Rust vault on both normal and fallback paths
- **Display Card Field Labels** — Added `toFormattedStringLocalized(l10n)` for i18n support across all default page display cards

## [1.5.0] - 2026-05-08

### Fixed

- **File Picker in Release Builds** — Added file access entitlements to DMG signing template for file_picker and image_picker
- **LLM Config Page** — Fixed redirecting to home page in release mode
- **Local Search & Scan Routes** — Opened to production builds

## [1.4.9] - 2026-05-08

### Fixed

- **LLM Config Page** — Fixed redirecting to home page in release mode (removed from `debugOnlyRoutes` guard)
- **Local Search & Scan Routes** — Opened to production builds

## [1.4.8] - 2026-05-08

### Added

- **Auto Language Detection** — OS locale auto-detection on first launch (Chinese OS → zh, all others → en)
- **Version Auto-Injection** — Version auto-injected into DMG builds, with update notification linking to GitHub Releases
- **935 ARB Keys** — Comprehensive i18n completion across all pages and widgets with 0 hardcoded strings remaining

### Fixed

- Date picker localization, password dialog width consistency, search empty state colors, untranslated "Vault" references

## [1.4.7] - 2026-05-06

### Added

- **Liquid Glass UI Overhaul** — Complete cross-platform UI redesign using liquid glass material design. All 20+ protected pages, AppBars, cards, buttons, dialogs, and sidebar now use glass-morphism effects with Notion+Anytype bright color palette
- **Login UI Refresh** — Redesigned login page with gradient background, decorative orbs, vertical centering, and hover effects on all interactive elements
- **Back Navigation** — SoloGlassAppBar now supports `backRoute` for proper back button behavior on deep-linked pages

### Fixed

- **Sensitivity Lock Enforcement** — Locking sensitive access now simultaneously enforces data masking and collapses all expanded history records
- **Sidebar Rename Bug** — Editing a custom page name no longer persists when navigating away; double-tap renamed to long-press for faster click response
- **Sidebar Drag Performance** — Cached descendant lookups during drag-and-drop and simplified drag placeholder to reduce jank
- **LLM Stats Persistence** — Skips LLM usage statistics persistence when vault is locked to prevent errors
- **Object Editor Character Counter** — Fixed character counter showing literal text instead of actual number
- **History Timestamp Alignment** — Full timestamps in history records are now right-aligned for consistency

### Refactored

- **Sensitivity Model Consolidation** — `sensitivity_models.dart` moved to `core/models/` for cleaner architecture
- **Scan Service Refactoring** — `local_search_service` now uses `FieldRegistry` as the single source of truth for field sensitivity levels
