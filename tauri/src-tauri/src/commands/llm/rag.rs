use crate::commands::vault_handle;
use crate::state::AppState;
use solosoul_vault::VaultStore;
use tauri::{Manager, State};

// =============================================================================
// RAG Embedding API (§RAG-3)
// =============================================================================

/// Normalize a vector to unit length.
use super::*;
fn normalize_vector(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}

/// Compute dot product of two vectors (assumes both are normalized).
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Source for embedding: cloud API or local ONNX model.
#[derive(Clone)]
enum EmbeddingSource {
    Cloud {
        base_url: String,
        api_key: String,
        model: String,
    },
    Local {
        model_id: String,
    },
}

/// Get the embedding source for the active account.
/// Checks local embedding preference first, then falls back to cloud provider.
fn get_embedding_source(
    vault: &VaultStore,
    account_id: &str,
    models_dir: &std::path::Path,
) -> Result<EmbeddingSource, String> {
    let config = load_config(vault, account_id)?;

    // 1. Check if local embedding is preferred and model is installed
    if config.use_local_embedding {
        if let Some(ref model_id) = config.local_embed_model_id {
            if crate::local_embed::is_model_installed(models_dir, model_id) {
                return Ok(EmbeddingSource::Local {
                    model_id: model_id.clone(),
                });
            }
        }
    }

    // 2. Fall back to cloud provider
    let active_id = config.active_provider_id.ok_or("No active provider")?;
    let providers = load_providers_with_keys(vault, account_id)?;
    let active = providers
        .into_iter()
        .find(|p| p.id == active_id)
        .ok_or("Active provider not found")?;

    if !active.is_enabled {
        return Err("Provider is disabled".to_string());
    }

    if matches!(active.api_type, ApiType::Anthropic) {
        return Err("Anthropic does not support embedding API".to_string());
    }

    let api_key = if active.api_key == "••••••••" {
        load_api_keys(vault, account_id)?
            .get(&active.id)
            .cloned()
            .unwrap_or_default()
    } else {
        active.api_key
    };

    let embedding_model = active
        .embedding_model
        .or_else(|| match active.name.to_lowercase().as_str() {
            n if n.contains("openai") => Some("text-embedding-3-small".into()),
            n if n.contains("ollama") => Some("nomic-embed-text".into()),
            n if n.contains("deepseek") => Some("text-embedding".into()),
            n if n.contains("alibaba") => Some("text-embedding-v3".into()),
            _ => None,
        })
        .ok_or("No embedding model configured for this provider")?;

    Ok(EmbeddingSource::Cloud {
        base_url: active.base_url,
        api_key,
        model: embedding_model,
    })
}

/// Call the embedding API (cloud or local) for a single text.
/// Returns normalized embedding vector.
async fn embed_text(
    source: EmbeddingSource,
    models_dir: std::path::PathBuf,
    text: String,
) -> Result<Vec<f32>, String> {
    match source {
        EmbeddingSource::Cloud {
            base_url,
            api_key,
            model,
        } => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| format!("Client: {}", e))?;

            let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
            let body = serde_json::json!({
                "input": text,
                "model": model,
                "encoding_format": "float"
            });

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Request to {} failed: {}", url, e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!(
                    "Embedding API HTTP {}: {}",
                    status,
                    body_text
                        .chars()
                        .take(MAX_PREVIEW_CHARS)
                        .collect::<String>()
                ));
            }

            let result: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Parse embedding response: {}", e))?;

            let embedding = result["data"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|obj| obj["embedding"].as_array())
                .ok_or("Invalid embedding response format")?;

            let mut vec: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();

            if vec.is_empty() {
                return Err("Empty embedding vector".to_string());
            }

            normalize_vector(&mut vec);
            Ok(vec)
        }
        EmbeddingSource::Local { model_id } => {
            let embedder = crate::local_embed::get_embedder_async(models_dir, model_id).await?;
            tokio::task::spawn_blocking(move || embedder.embed(&text))
                .await
                .map_err(|e| format!("Embedding task: {}", e))?
        }
    }
}

/// Batch embed multiple texts — sends all texts in a single API call for cloud providers.
async fn embed_texts(
    source: EmbeddingSource,
    models_dir: std::path::PathBuf,
    texts: Vec<String>,
) -> Result<Vec<Vec<f32>>, String> {
    match &source {
        EmbeddingSource::Local { model_id } => {
            let model_id = model_id.clone();
            let embedder = crate::local_embed::get_embedder_async(models_dir, model_id).await?;
            tokio::task::spawn_blocking(move || embedder.embed_batch(&texts))
                .await
                .map_err(|e| format!("Embedding batch task: {}", e))?
        }
        EmbeddingSource::Cloud {
            ref base_url,
            ref api_key,
            ref model,
        } => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .map_err(|e| format!("Client: {}", e))?;

            let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
            let body = serde_json::json!({
                "input": texts,
                "model": model,
                "encoding_format": "float"
            });

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Batch embedding request to {} failed: {}", url, e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!(
                    "Batch embedding API HTTP {}: {}",
                    status,
                    body_text
                        .chars()
                        .take(200)
                        .collect::<String>()
                ));
            }

            let result: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Parse batch embedding response: {}", e))?;

            let data = result["data"]
                .as_array()
                .ok_or("Invalid batch embedding response format: missing 'data'")?;

            // Sort by index to preserve input order
            let mut indexed: Vec<(usize, Vec<f32>)> = data
                .iter()
                .filter_map(|entry| {
                    let idx = entry["index"].as_u64()? as usize;
                    let emb = entry["embedding"]
                        .as_array()?
                        .iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect::<Vec<f32>>();
                    if emb.is_empty() { None } else { Some((idx, emb)) }
                })
                .collect();

            indexed.sort_by_key(|(idx, _)| *idx);

            let mut results: Vec<Vec<f32>> = indexed.into_iter().map(|(_, emb)| emb).collect();

            if results.len() != texts.len() {
                return Err(format!(
                    "Batch embedding returned {} results for {} texts",
                    results.len(),
                    texts.len()
                ));
            }

            // Normalize all vectors in place
            for v in &mut results {
                normalize_vector(v);
            }

            Ok(results)
        }
    }
}

/// Search guide chunks by vector similarity.
/// Falls back to keyword search if embedding is unavailable.
#[tauri::command]
pub async fn llm_search_guide_chunks(
    state: State<'_, AppState>,
    account_id: String,
    query: String,
    language: String,
    top_k: Option<usize>,
) -> Result<Vec<crate::commands::rag::GuideChunk>, String> {
    let top_k = top_k.unwrap_or(3);

    // 1. Load embedding source and existing chunks (sync block)
    let models_dir = state
        .handle
        .path()
        .resolve("models", tauri::path::BaseDirectory::LocalData)
        .map_err(|e| format!("Resolve models dir: {}", e))?;

    let (source, chunks) = {
        let vault = vault_handle(&state)?;

        let source = match get_embedding_source(&vault, &account_id, &models_dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[RAG] Embedding source error: {}, falling back to keyword search",
                    e
                );
                return fallback_keyword_search(&query, &language, top_k);
            }
        };

        let chunks = match vault.list_guide_embeddings() {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[RAG] Load embeddings failed: {}, falling back to keyword search",
                    e
                );
                return fallback_keyword_search(&query, &language, top_k);
            }
        };

        if chunks.is_empty() {
            eprintln!("[RAG] No embeddings found, falling back to keyword search");
            return fallback_keyword_search(&query, &language, top_k);
        }

        (source, chunks)
    };

    // 2. Embed query (async)
    let mut query_vec = match embed_text(source, models_dir, query.clone()).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "[RAG] Embed query failed: {}, falling back to keyword search",
                e
            );
            return fallback_keyword_search(&query, &language, top_k);
        }
    };
    normalize_vector(&mut query_vec);

    // 3. Compute similarities and return top-k
    let mut scored: Vec<(f32, solosoul_vault::GuideEmbeddingChunk)> = chunks
        .into_iter()
        .map(|chunk| {
            let sim = dot_product(&query_vec, &chunk.embedding);
            (sim, chunk)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut results = Vec::new();
    for (sim, chunk) in scored.into_iter().take(top_k) {
        let guide_title = crate::commands::rag::chunk_all_guides(&language)
            .ok()
            .and_then(|raws| raws.into_iter().find(|r| r.guide_id == chunk.guide_id))
            .map(|r| r.guide_title)
            .unwrap_or_else(|| chunk.guide_id.clone());

        results.push(crate::commands::rag::GuideChunk {
            guide_id: chunk.guide_id,
            guide_title,
            chunk_text: chunk.chunk_text,
            similarity: sim,
        });
    }

    if results.is_empty() {
        fallback_keyword_search(&query, &language, top_k)
    } else {
        Ok(results)
    }
}

/// Fallback to keyword-based guide search.
fn fallback_keyword_search(
    query: &str,
    language: &str,
    top_k: usize,
) -> Result<Vec<crate::commands::rag::GuideChunk>, String> {
    let guides = find_relevant_guides_internal(query, language)?;
    let mut results = Vec::new();
    for guide in guides.into_iter().take(top_k) {
        results.push(crate::commands::rag::GuideChunk {
            guide_id: guide.id.clone(),
            guide_title: guide.title.clone(),
            chunk_text: guide.content,
            similarity: 0.5, // placeholder similarity for fallback
        });
    }
    Ok(results)
}

/// Rebuild all guide embeddings. Clears existing and re-creates.
#[tauri::command]
pub async fn llm_rebuild_guide_embeddings(
    state: State<'_, AppState>,
    account_id: String,
    language: String,
) -> Result<usize, String> {
    let models_dir = state
        .handle
        .path()
        .resolve("models", tauri::path::BaseDirectory::LocalData)
        .map_err(|e| format!("Resolve models dir: {}", e))?;

    // 1. Extract embedding source and chunk guides (sync, vault guard released after this block)
    let (source, raw_chunks) = {
        let vault = vault_handle(&state)?;

        let source = get_embedding_source(&vault, &account_id, &models_dir)?;
        let raw_chunks = crate::commands::rag::chunk_all_guides(&language)?;
        if raw_chunks.is_empty() {
            return Ok(0);
        }
        vault
            .clear_guide_embeddings()
            .map_err(|e| format!("Clear embeddings: {}", e))?;
        (source, raw_chunks)
    };

    let model_name = match &source {
        EmbeddingSource::Cloud { model, .. } => model.clone(),
        EmbeddingSource::Local { model_id } => model_id.clone(),
    };

    // 2. Batch embed all chunks (async)
    let texts: Vec<String> = raw_chunks.iter().map(|c| c.text.clone()).collect();
    let embeddings = embed_texts(source, models_dir, texts).await?;

    // 3. Store in vault (sync)
    let count = {
        let vault = vault_handle(&state)?;

        let now = chrono::Utc::now().to_rfc3339();
        for (i, (raw, mut vec)) in raw_chunks.into_iter().zip(embeddings).enumerate() {
            normalize_vector(&mut vec);
            let chunk = solosoul_vault::GuideEmbeddingChunk {
                id: format!("{}_{}", raw.guide_id, raw.chunk_index),
                guide_id: raw.guide_id,
                chunk_index: raw.chunk_index as i32,
                chunk_text: raw.text,
                embedding: vec.to_vec(),
                model: model_name.clone(),
                created_at: now.clone(),
            };
            vault
                .save_guide_embedding(&chunk)
                .map_err(|e| format!("Save embedding {}: {}", i, e))?;
        }

        crate::commands::rag::mark_rebuilt(&vault, &language)?;
        vault.count_guide_embeddings()?
    };

    Ok(count)
}

/// Check if embedding is available (cloud or local).
#[tauri::command]
pub async fn llm_check_embedding_available(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<bool, String> {
    let models_dir = state
        .handle
        .path()
        .resolve("models", tauri::path::BaseDirectory::LocalData)
        .map_err(|e| format!("Resolve models dir: {}", e))?;

    let source = {
        let vault = vault_handle(&state)?;
        get_embedding_source(&vault, &account_id, &models_dir)
    };

    match source {
        Ok(EmbeddingSource::Local { model_id }) => {
            // Local model: just check if it's installed and can load
            match crate::local_embed::get_embedder_async(models_dir, model_id).await {
                Ok(_) => Ok(true),
                Err(e) => {
                    eprintln!("[RAG] Local embedding not available: {}", e);
                    Ok(false)
                }
            }
        }
        Ok(EmbeddingSource::Cloud {
            base_url,
            api_key,
            model,
        }) => {
            // Try a test embedding call with a dummy text
            match embed_text(
                EmbeddingSource::Cloud {
                    base_url,
                    api_key,
                    model,
                },
                models_dir,
                "test".into(),
            )
            .await
            {
                Ok(_) => Ok(true),
                Err(e) => {
                    eprintln!("[RAG] Embedding availability check failed: {}", e);
                    Ok(false)
                }
            }
        }
        Err(e) => {
            eprintln!("[RAG] No embedding source: {}", e);
            Ok(false)
        }
    }
}

/// Ensure guide embeddings are built on app startup.
/// Called from app setup. Non-blocking, errors are logged only.
#[allow(clippy::await_holding_lock)]
pub async fn ensure_guide_embeddings_built(state: &AppState, account_id: &str, language: &str) {
    let result = async {
        let models_dir = state
            .handle
            .path()
            .resolve("models", tauri::path::BaseDirectory::LocalData)
            .map_err(|e| format!("Resolve models dir: {}", e))?;

        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref();

        if !crate::commands::rag::needs_rebuild(vault, language)? {
            return Ok::<(), String>(());
        }

        let source = match get_embedding_source(vault, account_id, &models_dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[RAG] Cannot build embeddings: {}", e);
                return Ok(());
            }
        };

        let model_name = match &source {
            EmbeddingSource::Cloud { model, .. } => model.clone(),
            EmbeddingSource::Local { model_id } => model_id.clone(),
        };

        let raw_chunks = crate::commands::rag::chunk_all_guides(language)?;
        if raw_chunks.is_empty() {
            return Ok(());
        }

        vault.clear_guide_embeddings().map_err(|e| e.to_string())?;
        drop(vg);
        drop(svc);

        let texts: Vec<String> = raw_chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = embed_texts(source, models_dir, texts).await?;

        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref();

        let now = chrono::Utc::now().to_rfc3339();
        for (raw, mut vec) in raw_chunks.into_iter().zip(embeddings) {
            normalize_vector(&mut vec);
            let chunk = solosoul_vault::GuideEmbeddingChunk {
                id: format!("{}_{}", raw.guide_id, raw.chunk_index),
                guide_id: raw.guide_id,
                chunk_index: raw.chunk_index as i32,
                chunk_text: raw.text,
                embedding: vec.to_vec(),
                model: model_name.clone(),
                created_at: now.clone(),
            };
            vault
                .save_guide_embedding(&chunk)
                .map_err(|e| e.to_string())?;
        }

        crate::commands::rag::mark_rebuilt(vault, language)?;
        let count = vault.count_guide_embeddings()?;
        eprintln!("[RAG] Built {} guide embeddings", count);
        Ok(())
    }
    .await;

    if let Err(e) = result {
        eprintln!("[RAG] Failed to build embeddings: {}", e);
    }
}
