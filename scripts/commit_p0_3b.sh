#!/bin/bash
cd /Users/zzc/PycharmProjects/SoloSoul_code
git add tauri/src/components/plugin/shared/ExpandableSection.tsx tauri/src/components/plugin/shared/ExpandableSection.module.css
git commit -m 'fix(P0-3b): replace ExpandableSection with native <details>/<summary> (-137 lines)

Replace custom state-based collapsible with native <details> element.
Actions inside <summary> use [data-actions] + closest() detection to
prevent details toggle via e.preventDefault(). Hide default ::marker.
API unchanged: all 3 callers (PluginLogSection, PluginResultSection,
WatermarkPluginConfig) need no modifications.'
git push origin master
