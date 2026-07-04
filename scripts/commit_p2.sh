#!/bin/bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

git add -A
git commit -m "refactor: P2 ponytail audit — CLI wrapper inlining & dedup

- byte_position extracted to shared crate::util, deduped across 3 widgets
- CommandPalette::should_show inlined into should_render
- trigger_debug_log_export inlined (replaced with direct debug_log call)
- parse_value extracted to commands/mod.rs, deduped settings+profile
- update_profile_preference extracted to commands/mod.rs, deduped settings+security
- Removed unused Map import from security.rs"

git push origin master
echo "✅ P2 pushed"
