#!/bin/bash
cd /Users/zzc/PycharmProjects/SoloSoul_code
git add tauri/crates/solosoul-crypto/src/aes.rs tauri/crates/solosoul-crypto/src/cipher.rs tauri/src-tauri/src/commands/crypto.rs tauri/crates/solosoul-vault/src/encryption.rs
git commit -m 'fix(P0-4): unify AES error types - replace String with CipherError

Add CipherError::BlobFormat variant for SOLO blob-specific errors.
Change all aes.rs return types from Result<..., String> to
Result<..., CipherError>. Add key_err/enc_err/fmt_err helpers.
Update callers in crypto.rs and solosoul-vault/encryption.rs with
.map_err(|e| e.to_string()). legacy.rs and fs.rs use format!("{e}")
which works with CipherError (implements Display via thiserror).'
git push origin master
