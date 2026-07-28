#!/bin/bash
cd /Users/zzc/PycharmProjects/SoloSoul
git add tauri/src-tauri/src/lib.rs
git rm -r tauri/src-tauri/src/db
git commit -m 'fix(P0-2): delete deprecated db/ module (-221 lines)

Remove migrations.rs (only used by its own tests), connection.rs (empty),
and mod.rs. The real schema is managed by solosoul-vault crate.
remove pub mod db from lib.rs.'
git push origin main
