//! RAG (Retrieval-Augmented Generation) module for Help Doc vector search.
//! Document chunking, embedding pre-build, and content hash management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export resource_path from llm module for guide file access
use super::llm::{resource_path, GuideIndexEntry, resolve_language, resolve_title};

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
    let index = super::llm::load_guide_index()?;
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
    let index = super::llm::load_guide_index()?;
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
/// Returns true if the table is empty or content hash has changed.
pub fn needs_rebuild(vault: &solosoul_vault::VaultStore, language: &str) -> Result<bool, String> {
    let count = vault.count_guide_embeddings()?;
    if count == 0 {
        return Ok(true);
    }

    let current_hash = compute_content_hash(language)?;
    let stored_hash: Option<String> = get_sys_config(vault, "guide_embeddings_content_hash")?;

    match stored_hash {
        Some(h) if h == current_hash => Ok(false),
        _ => Ok(true),
    }
}

/// Update the content hash and build timestamp in sys_config.
pub fn mark_rebuilt(vault: &solosoul_vault::VaultStore, language: &str) -> Result<(), String> {
    let hash = compute_content_hash(language)?;
    set_sys_config(vault, "guide_embeddings_content_hash", &hash)?;
    let now = chrono::Utc::now().to_rfc3339();
    set_sys_config(vault, "guide_embeddings_built_at", &now)?;
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
        if line.starts_with("## ") {
            // Flush previous section
            if !current_section_lines.is_empty() {
                let section_text = current_section_lines.join("\n").trim().to_string();
                if !section_text.is_empty() {
                    let mut section_chunks = split_section(&section_text, doc_title, &current_section_title, guide_id, chunks.len());
                    chunks.append(&mut section_chunks);
                }
            }
            current_section_title = line[3..].trim().to_string();
            current_section_lines.clear();
        } else {
            current_section_lines.push(line);
        }
    }

    // Flush last section
    if !current_section_lines.is_empty() {
        let section_text = current_section_lines.join("\n").trim().to_string();
        if !section_text.is_empty() {
            let mut section_chunks = split_section(&section_text, doc_title, &current_section_title, guide_id, chunks.len());
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
/// Handles both `<!--NAME-->` (single-line start marker) and `<!--NAME` (multi-line start).
fn clean_special_blocks(content: &str) -> String {
    let mut result = String::new();
    let mut in_block = false;
    let mut block_name = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<!--") && !trimmed.starts_with("<!--/") {
            // Block start like <!--TIP--> or <!--TIP
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
/// Adds 50-char overlap between adjacent chunks.
const MAX_CHUNK_LEN: usize = 800;
const OVERLAP_LEN: usize = 50;

fn split_section(text: &str, doc_title: &str, section_title: &str, guide_id: &str, start_index: usize) -> Vec<RawChunk> {
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

        // Detect code block boundaries
        let code_fence_count = para.lines().filter(|l| l.trim().starts_with("```")).count();
        if code_fence_count % 2 == 1 {
            in_code_block = !in_code_block;
        }

        // Detect table (lines starting with |)
        let is_table_line = para.lines().all(|l| l.trim().starts_with('|') || l.trim().is_empty());
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

        // If adding this paragraph would exceed limit, and we're not inside a code block/table,
        // flush current chunk and start new one with overlap
        if current_chunk.len() + para_with_sep.len() > MAX_CHUNK_LEN && !in_code_block && !in_table {
            let idx = start_index + chunks.len();
            chunks.push(RawChunk {
                guide_id: guide_id.to_string(),
                guide_title: doc_title.to_string(),
                chunk_index: idx,
                text: current_chunk.trim().to_string(),
            });

            // Start new chunk with prefix + overlap from end of previous
            let overlap = get_overlap(&current_chunk, OVERLAP_LEN);
            current_chunk = format!("{}\n\n{}", prefix, overlap);
        }

        current_chunk.push_str(&para_with_sep);
    }

    // Flush remaining
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
    // Find a good boundary (newline) near the desired overlap length
    let start = text.len() - len;
    let mut pos = start;
    // Look for next newline after start position
    if let Some(next_nl) = text[start..].find('\n') {
        pos = start + next_nl + 1;
    }
    text[pos..].to_string()
}

// ── sys_config helpers ────────────────────────────────────────

fn get_sys_config(vault: &solosoul_vault::VaultStore, key: &str) -> Result<Option<String>, String> {
    vault.get_sys_config(key)
}

fn set_sys_config(vault: &solosoul_vault::VaultStore, key: &str, value: &str) -> Result<(), String> {
    vault.set_sys_config(key, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_markdown_simple() {
        let md = "# Title\n\nIntro text.\n\n## Section A\n\nContent A.\n\n## Section B\n\nContent B.\n";
        let chunks = chunk_markdown(md, "Doc Title", "guide_1");
        // Intro text before any ## becomes a chunk too
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
        let md = "Before\n\n<!--TIP-->\nTip content line 1\nTip content line 2\n<!--/TIP-->\n\nAfter\n";
        let cleaned = clean_special_blocks(md);
        assert!(cleaned.contains("Before"));
        assert!(cleaned.contains("After"));
        assert!(!cleaned.contains("Tip content line 1"));
        assert!(!cleaned.contains("<!--TIP-->"));
    }

    #[test]
    fn test_chunk_respects_max_length() {
        // Create many short paragraphs separated by blank lines
        let mut long_content = String::new();
        for i in 0..30 {
            long_content.push_str(&format!("Paragraph {} with enough text to make it long.\n\n", i));
        }
        let md = format!("# Title\n\n## Section\n\n{}\n", long_content);
        let chunks = chunk_markdown(&md, "Doc", "g1");
        assert!(chunks.len() >= 2, "Long section should be split into multiple chunks, got {}", chunks.len());
        for chunk in &chunks {
            assert!(chunk.text.len() <= MAX_CHUNK_LEN + 200,
                "Chunk should respect max length: got {} chars", chunk.text.len());
        }
    }
}
