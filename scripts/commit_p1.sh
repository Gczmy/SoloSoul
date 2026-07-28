#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# Stage all changes
git add -A

git commit -m "refactor: P1 ponytail audit — deps, stdlib, re-exports, dead code

P1 items completed:
- P1-1: once_cell::sync::Lazy → std::sync::LazyLock (3 files)
- P1-2: Remove walkdir dep (zero references)
- P1-3: Replace pdf-extract with pdfium-render in extract_pdf_text
- P1-4: Remove SecureBytes/SecureString/secure_wipe, keep only secure_compare
- P1-5: Simplify CliError → String type alias, remove thiserror dep
- P1-6: Replace hex::encode with format!() in CLI sync.rs
- P1-7: Replace uuid::Uuid::new_v4() with atomic counters in CLI tests
- P1-8: Remove tokio-util dep (zero references)
- P1-9: Replace unicode-segmentation graphemes with str::chars()
- P1-10: Delete services/vault_service.rs re-export, update callers
- P1-11: Delete plugin/field.rs re-export, update callers
- P1-12: Delete empty ipc/mod.rs
- P1-13: Delete ATTACHMENT_STREAMING_THRESHOLD dead constants
- P1-15: Slim settings_language_select/theme_select → delegate to settings_select

Net: -7 deps (walkdir, once_cell, unicode-segmentation, thiserror, hex, uuid, tokio-util)"

git push origin main
