#!/bin/bash
cd /Users/zzc/PycharmProjects/SoloSoul
git add tauri/src-tauri/src/commands/llm/rag.rs tauri/src-tauri/src/commands/mod.rs docs/P0.5-PONYTAIL-AUDIT-REPORT.md
git rm tauri/src-tauri/src/commands/rag.rs
git commit -m 'fix(P0-1): merge commands/rag.rs into commands/llm/rag.rs (-384 lines)

Move GuideChunk, RawChunk, chunk_all_guides, compute_content_hash,
needs_rebuild, mark_rebuilt, chunk_markdown, and tests from the
standalone commands/rag.rs into commands/llm/rag.rs where the embedding
API already lives. Update all crate::commands::rag::xxx references to
direct calls since they are now in the same module. Delete the old file
and remove pub mod rag from commands/mod.rs.'
git push origin main
