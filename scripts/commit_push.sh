#!/bin/bash
set -e
cd /Users/zzc/PycharmProjects/SoloSoul_code

git add -A
git commit -m 'feat(import): add KeepBoth strategy with locale-based suffix, ID rewriting, and per-object conflict UI' \
          -m 'Import conflict handling improvements:
- Add KeepBoth strategy: conflict objects get new UUID + auto-rename suffix (zh: （导入）, en: (Imported))
- Classify conflicts as Identical / RenamedLocal
- Per-object strategy override via object_strategies
- rewrite_id_references for parent_id, children_ids, relation properties
- Fix attachment directory path for KeepBoth objects
- unique_object_name optimized with HashSet
- Frontend conflict detail display + per-object strategy selector
- Locale passed from frontend i18n.language

Also:
- Add expiry-guardian to plugin filter list'

git push origin master
echo "Done."
