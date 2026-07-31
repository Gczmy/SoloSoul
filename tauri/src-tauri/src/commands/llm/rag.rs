use crate::commands::vault_handle;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
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
                    body_text.chars().take(200).collect::<String>()
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
                    if emb.is_empty() {
                        None
                    } else {
                        Some((idx, emb))
                    }
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
) -> Result<Vec<GuideChunk>, String> {
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

    // 一次性读取指南文件并建立 guide_id → title 映射，避免在 top_k 循环内
    // 反复 chunk_all_guides（每条结果都把全部指南文件完整读一遍）。
    let titles: std::collections::HashMap<String, String> = chunk_all_guides(&language)
        .map(|raws| {
            raws.into_iter()
                .map(|r| (r.guide_id, r.guide_title))
                .collect()
        })
        .unwrap_or_default();

    let mut results = Vec::new();
    for (sim, chunk) in scored.into_iter().take(top_k) {
        let guide_title = titles
            .get(&chunk.guide_id)
            .cloned()
            .unwrap_or_else(|| chunk.guide_id.clone());

        results.push(GuideChunk {
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
) -> Result<Vec<GuideChunk>, String> {
    let guides = find_relevant_guides_internal(query, language)?;
    let mut results = Vec::new();
    for guide in guides.into_iter().take(top_k) {
        results.push(GuideChunk {
            guide_id: guide.id.clone(),
            guide_title: guide.title.clone(),
            chunk_text: guide.content,
            similarity: 0.5, // placeholder similarity for fallback
        });
    }
    Ok(results)
}

// ── Guide chunking (moved from commands/rag.rs) ────────────────

/// A single chunk returned to the frontend for context injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideChunk {
    pub guide_id: String,
    pub guide_title: String,
    pub chunk_text: String,
    pub similarity: f32,
}

/// Internal representation of a raw document chunk before embedding.
#[derive(Debug, Clone)]
pub struct RawChunk {
    pub guide_id: String,
    pub guide_title: String,
    pub chunk_index: usize,
    pub text: String,
}

/// Parse all guides into semantic chunks.
pub fn chunk_all_guides(language: &str) -> Result<Vec<RawChunk>, String> {
    let index = load_guide_index()?;
    let mut all_chunks = Vec::new();

    for entry in &index.guides {
        let lang = resolve_language(&entry.files, language);
        let rel_path = format!("docs/guides/{}", entry.files.get(&lang).ok_or("No file")?);
        let path = resource_path(&rel_path);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[RAG] skip guide {}: {}", entry.id, e);
                continue;
            }
        };

        let title = resolve_title(&entry.title, language);
        let chunks = chunk_markdown(&content, &title, &entry.id);
        all_chunks.extend(chunks);
    }

    Ok(all_chunks)
}

/// Compute a content hash of all guide documents for change detection.
pub fn compute_content_hash(language: &str) -> Result<String, String> {
    let index = load_guide_index()?;
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();

    for entry in &index.guides {
        let lang = resolve_language(&entry.files, language);
        let rel_path = format!("docs/guides/{}", entry.files.get(&lang).ok_or("No file")?);
        let path = resource_path(&rel_path);
        if let Ok(content) = std::fs::read_to_string(&path) {
            hasher.update(content.as_bytes());
        }
    }

    let digest = hasher.finalize();
    Ok(format!("{:x}", digest))
}

/// Check if embeddings need to be rebuilt.
pub fn needs_rebuild(vault: &solosoul_vault::VaultStore, language: &str) -> Result<bool, String> {
    let count = vault.count_guide_embeddings()?;
    if count == 0 {
        return Ok(true);
    }

    let current_hash = compute_content_hash(language)?;
    let stored_hash: Option<String> = vault.get_sys_config("guide_embeddings_content_hash")?;

    match stored_hash {
        Some(h) if h == current_hash => Ok(false),
        _ => Ok(true),
    }
}

/// Update the content hash and build timestamp in sys_config.
pub fn mark_rebuilt(vault: &solosoul_vault::VaultStore, language: &str) -> Result<(), String> {
    let hash = compute_content_hash(language)?;
    vault.set_sys_config("guide_embeddings_content_hash", &hash)?;
    let now = chrono::Utc::now().to_rfc3339();
    vault.set_sys_config("guide_embeddings_built_at", &now)?;
    Ok(())
}

// ── Markdown chunking ─────────────────────────────────────────

fn chunk_markdown(content: &str, doc_title: &str, guide_id: &str) -> Vec<RawChunk> {
    let cleaned = clean_special_blocks(content);
    let lines: Vec<&str> = cleaned.lines().collect();

    let mut chunks = Vec::new();
    let mut current_section_title = String::new();
    let mut current_section_lines: Vec<&str> = Vec::new();

    for line in &lines {
        if let Some(title) = line.strip_prefix("## ") {
            // Flush previous section
            if !current_section_lines.is_empty() {
                let section_text = current_section_lines.join("\n").trim().to_string();
                if !section_text.is_empty() {
                    let mut section_chunks = split_section(
                        &section_text,
                        doc_title,
                        &current_section_title,
                        guide_id,
                        chunks.len(),
                    );
                    chunks.append(&mut section_chunks);
                }
            }
            current_section_title = title.trim().to_string();
            current_section_lines.clear();
        } else {
            current_section_lines.push(line);
        }
    }

    // Flush last section
    if !current_section_lines.is_empty() {
        let section_text = current_section_lines.join("\n").trim().to_string();
        if !section_text.is_empty() {
            let mut section_chunks = split_section(
                &section_text,
                doc_title,
                &current_section_title,
                guide_id,
                chunks.len(),
            );
            chunks.append(&mut section_chunks);
        }
    }

    // If no ## sections found, treat entire doc as one chunk
    if chunks.is_empty() {
        let text = cleaned.trim().to_string();
        if !text.is_empty() {
            chunks.push(RawChunk {
                guide_id: guide_id.to_string(),
                guide_title: doc_title.to_string(),
                chunk_index: 0,
                text: format!("[{}]\n\n{}", doc_title, text),
            });
        }
    }

    chunks
}

/// Remove special HTML comment blocks that are UI hints, not content.
fn clean_special_blocks(content: &str) -> String {
    let mut result = String::new();
    let mut in_block = false;
    let mut block_name = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<!--") && !trimmed.starts_with("<!--/") {
            in_block = true;
            let inner = trimmed.trim_start_matches("<!--").trim_end_matches("-->");
            block_name = inner.to_string();
            continue;
        }
        if trimmed.starts_with("<!--/") && trimmed.ends_with("-->") {
            let end_name = trimmed.trim_start_matches("<!--/").trim_end_matches("-->");
            if in_block && end_name == block_name {
                in_block = false;
                block_name.clear();
                continue;
            }
        }
        if in_block {
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Split a section into chunks if it exceeds MAX_CHUNK_LEN.
const MAX_CHUNK_LEN: usize = 800;
const OVERLAP_LEN: usize = 50;

fn split_section(
    text: &str,
    doc_title: &str,
    section_title: &str,
    guide_id: &str,
    start_index: usize,
) -> Vec<RawChunk> {
    let prefix = if section_title.is_empty() {
        format!("[{}]", doc_title)
    } else {
        format!("[{}] > [{}]", doc_title, section_title)
    };

    let full_text = format!("{}\n\n{}", prefix, text);

    if full_text.len() <= MAX_CHUNK_LEN {
        return vec![RawChunk {
            guide_id: guide_id.to_string(),
            guide_title: doc_title.to_string(),
            chunk_index: start_index,
            text: full_text,
        }];
    }

    // Split by paragraphs, respecting code blocks and tables
    let mut chunks = Vec::new();
    let mut current_chunk = prefix.clone();
    current_chunk.push_str("\n\n");
    let mut in_code_block = false;
    let mut in_table = false;

    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }

        let code_fence_count = para.lines().filter(|l| l.trim().starts_with("```")).count();
        if code_fence_count % 2 == 1 {
            in_code_block = !in_code_block;
        }

        let is_table_line = para
            .lines()
            .all(|l| l.trim().starts_with('|') || l.trim().is_empty());
        if is_table_line && !para.lines().next().unwrap_or("").trim().starts_with("```") {
            in_table = true;
        } else if in_table && !is_table_line {
            in_table = false;
        }

        let para_with_sep = if current_chunk.ends_with("\n\n") {
            para.to_string()
        } else {
            format!("\n\n{}", para)
        };

        if current_chunk.len() + para_with_sep.len() > MAX_CHUNK_LEN && !in_code_block && !in_table
        {
            let idx = start_index + chunks.len();
            chunks.push(RawChunk {
                guide_id: guide_id.to_string(),
                guide_title: doc_title.to_string(),
                chunk_index: idx,
                text: current_chunk.trim().to_string(),
            });

            let overlap = get_overlap(&current_chunk, OVERLAP_LEN);
            current_chunk = format!("{}\n\n{}", prefix, overlap);
        }

        current_chunk.push_str(&para_with_sep);
    }

    let trimmed = current_chunk.trim();
    if trimmed.len() > prefix.len() + 2 {
        let idx = start_index + chunks.len();
        chunks.push(RawChunk {
            guide_id: guide_id.to_string(),
            guide_title: doc_title.to_string(),
            chunk_index: idx,
            text: trimmed.to_string(),
        });
    }

    chunks
}

fn get_overlap(text: &str, len: usize) -> String {
    if text.len() <= len {
        return text.to_string();
    }
    let start = text.len() - len;
    let mut pos = start;
    if let Some(next_nl) = text[start..].find('\n') {
        pos = start + next_nl + 1;
    }
    text[pos..].to_string()
}

// ── End moved from commands/rag.rs ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_markdown_simple() {
        let md =
            "# Title\n\nIntro text.\n\n## Section A\n\nContent A.\n\n## Section B\n\nContent B.\n";
        let chunks = chunk_markdown(md, "Doc Title", "guide_1");
        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].text.contains("Intro text"));
        assert!(chunks[1].text.contains("Section A"));
        assert!(chunks[2].text.contains("Section B"));
        assert!(chunks[1].text.starts_with("[Doc Title] > [Section A]"));
    }

    #[test]
    fn test_chunk_markdown_no_h2() {
        let md = "# Title\n\nOnly intro, no sections.\n";
        let chunks = chunk_markdown(md, "Doc Title", "guide_1");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("Only intro"));
    }

    #[test]
    fn test_clean_special_blocks() {
        let md =
            "Before\n\n<!--TIP-->\nTip content line 1\nTip content line 2\n<!--/TIP-->\n\nAfter\n";
        let cleaned = clean_special_blocks(md);
        assert!(cleaned.contains("Before"));
        assert!(cleaned.contains("After"));
        assert!(!cleaned.contains("Tip content line 1"));
        assert!(!cleaned.contains("<!--TIP-->"));
    }

    #[test]
    fn test_chunk_respects_max_length() {
        let mut long_content = String::new();
        for i in 0..30 {
            long_content.push_str(&format!(
                "Paragraph {} with enough text to make it long.\n\n",
                i
            ));
        }
        let md = format!("# Title\n\n## Section\n\n{}\n", long_content);
        let chunks = chunk_markdown(&md, "Doc", "g1");
        assert!(
            chunks.len() >= 2,
            "Long section should be split into multiple chunks, got {}",
            chunks.len()
        );
        for chunk in &chunks {
            assert!(
                chunk.text.len() <= MAX_CHUNK_LEN + 200,
                "Chunk should respect max length: got {} chars",
                chunk.text.len()
            );
        }
    }
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
        let raw_chunks = chunk_all_guides(&language)?;
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
        // P051: 先收集全部 chunk，再单事务批量写入，避免逐条 autocommit + fsync
        let chunks: Vec<solosoul_vault::GuideEmbeddingChunk> = raw_chunks
            .into_iter()
            .zip(embeddings)
            .map(|(raw, mut vec)| {
                normalize_vector(&mut vec);
                solosoul_vault::GuideEmbeddingChunk {
                    id: format!("{}_{}", raw.guide_id, raw.chunk_index),
                    guide_id: raw.guide_id,
                    chunk_index: raw.chunk_index as i32,
                    chunk_text: raw.text,
                    embedding: vec.to_vec(),
                    model: model_name.clone(),
                    created_at: now.clone(),
                }
            })
            .collect();
        vault
            .save_guide_embeddings(&chunks)
            .map_err(|e| format!("Save embeddings ({} chunks): {}", chunks.len(), e))?;

        mark_rebuilt(&vault, &language)?;
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
