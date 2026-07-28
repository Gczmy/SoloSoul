#!/bin/bash
cd /Users/zzc/PycharmProjects/SoloSoul
git add tauri/src/components/ui/Dialog.tsx tauri/src/components/ui/Dialog.module.css tauri/src/components/ui/Dialog.test.tsx tauri/src/test/setup.ts
git commit -m 'fix(P0-3c): replace Dialog with native <dialog> element (-93 lines)

Use native <dialog> with showModal()/close() instead of custom overlay +
inner div with Escape keydown listener and click-outside detection.
Move backdrop styles to ::backdrop pseudo-element.
Add HTMLDialogElement polyfill for jsdom in test/setup.ts.
Update Dialog.test.tsx for new <dialog> API.'
git push origin main
