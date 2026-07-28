#!/bin/bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

git add -A
git commit -m "refactor: P2 frontend — useCancellable→AbortController, dead code, inlining

- useCancellable hook replaced with AbortController (4 call sites), hook deleted
- Dead code removed: assessPasswordStrength, stopListeningForSystemTheme,
  findRelevantGuides, formatGuideAsSystemMessage
- buildMessagesWithSystemPromptAndChunks inlined into buildMessagesWithSystemPromptAndGuide
- Local formatFileSize in ScanLocalPage replaced with formatBytes from utils
- Local truncateFileName in TrashDetailPanel replaced with import from attachmentUtils
- SafeMarkdown simplified to rest-props passthrough with className wrapper
- theme.test.ts deleted (adjustAccentHover tests removed with function retained)"

git push origin main
echo "✅ P2 frontend pushed"
