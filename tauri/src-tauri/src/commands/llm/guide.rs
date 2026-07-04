use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Help Guide Retrieval (§7)
// =============================================================================

use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::{Mutex, OnceLock};

/// 应用资源目录，由 `lib.rs` 在 setup 阶段通过 `app.path().resource_dir()` 初始化。
/// 生产模式下使用它可正确解析 Tauri v2 打包后的资源路径。
use super::*;
pub static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideTitle {
    pub zh: String,
    pub en: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideCategoryMeta {
    pub id: String,
    pub title: GuideTitle,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideIndexEntry {
    pub id: String,
    pub title: GuideTitle,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub order: u32,
    pub keywords: Vec<String>,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideIndex {
    pub guides: Vec<GuideIndexEntry>,
    #[serde(default)]
    pub categories: Vec<GuideCategoryMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideContent {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// 资源文件路径解析：开发模式从 src-tauri/resources/ 读取，生产模式从 app bundle 读取
pub fn resource_path(rel: &str) -> PathBuf {
    // 优先使用 Tauri 在 setup 阶段解析的资源目录，保证生产包路径正确。
    if let Some(dir) = RESOURCE_DIR.get() {
        let path = dir.join(rel);
        eprintln!("[resource_path] resource_dir: {:?}", path);
        return path;
    }

    if cfg!(debug_assertions) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(rel);
        eprintln!("[resource_path] debug fallback: {:?}", path);
        path
    } else {
        let exe = std::env::current_exe().unwrap_or_default();
        let exe_dir = exe.parent().unwrap_or(&exe).to_path_buf();
        #[cfg(target_os = "macos")]
        {
            // macOS app bundle: SoloSoul.app/Contents/MacOS/SoloSoul → ../Resources
            let path = exe_dir.join("../Resources").join(rel);
            eprintln!("[resource_path] release fallback macOS: {:?}", path);
            path
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Windows / Linux 兜底：资源与可执行文件同目录或 resources 子目录
            let path = exe_dir.join(rel);
            eprintln!("[resource_path] release fallback: {:?}", path);
            path
        }
    }
}

/// 缓存的指南索引
static GUIDE_INDEX_CACHE: LazyLock<Mutex<Option<GuideIndex>>> = LazyLock::new(|| Mutex::new(None));

/// 指南摘要缓存：guideId -> 前 200 字摘要（用于 AI 快速匹配）
static GUIDE_SUMMARY_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 缓存的全文搜索索引
static SEARCH_INDEX_CACHE: LazyLock<Mutex<Option<SearchIndex>>> =
    LazyLock::new(|| Mutex::new(None));

/// 获取缓存内容，容忍毒化锁（poisoned lock recovery）
fn get_index_cache() -> Option<GuideIndex> {
    let guard = GUIDE_INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    guard.clone()
}

fn set_index_cache(index: GuideIndex) {
    let mut guard = GUIDE_INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(index);
}

fn get_summary_cache() -> HashMap<String, String> {
    let guard = GUIDE_SUMMARY_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    guard.clone()
}

fn set_summary_cache(summaries: HashMap<String, String>) {
    let mut guard = GUIDE_SUMMARY_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *guard = summaries;
}

fn get_search_index_cache() -> Option<SearchIndex> {
    let guard = SEARCH_INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    guard.clone()
}

fn set_search_index_cache(index: SearchIndex) {
    let mut guard = SEARCH_INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(index);
}

/// 如果摘要缓存为空，则按需预加载所有指南摘要。
/// 该操作从 `load_guide_index` 中剥离，避免帮助文档首页因读取全部文件而长时间显示加载中。
fn ensure_summary_cache(index: &GuideIndex) {
    let cache = get_summary_cache();
    if !cache.is_empty() {
        return;
    }

    let mut summaries = HashMap::new();
    for guide in &index.guides {
        let lang = guide
            .files
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "en".to_string());
        if let Some(file) = guide.files.get(&lang) {
            let file_path = resource_path(&format!("docs/guides/{}", file));
            if let Ok(text) = std::fs::read_to_string(&file_path) {
                let summary = if text.len() > MAX_GUIDE_SUMMARY_BYTES {
                    // 找到不超过 MAX_GUIDE_SUMMARY_BYTES 字节的最近合法字符边界（避免在中文字符中间切片 panic）
                    let mut end = MAX_GUIDE_SUMMARY_BYTES;
                    while !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    let cut = &text[..end];
                    match cut.rfind('\n') {
                        Some(pos) => text[..pos].to_string(),
                        None => cut.to_string(),
                    }
                } else {
                    text
                };
                summaries.insert(guide.id.clone(), summary);
            }
        }
    }
    set_summary_cache(summaries);
}

pub fn load_guide_index() -> Result<GuideIndex, String> {
    if let Some(idx) = get_index_cache() {
        return Ok(idx);
    }

    let path = resource_path("docs/guides/index.json");
    eprintln!("[load_guide_index] reading from {:?}", path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide index at {:?}: {}", path, e))?;
    let index: GuideIndex = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse guide index: {}", e))?;

    set_index_cache(index.clone());
    Ok(index)
}

/// 分词 + 停用词过滤（简化版）
fn tokenize_query(query: &str) -> Vec<String> {
    let lowered = query.to_lowercase();
    // 中文按字符分，英文按空格分
    let tokens: Vec<String> = lowered
        .split_whitespace()
        .flat_map(|s| {
            if s.chars().any(|c| c as u32 >= 0x4E00 && c as u32 <= 0x9FFF) {
                // 含中文字符：每个中文字符单独作为一个 token，英文部分整体保留
                s.chars()
                    .filter(|c| !is_stop_char(*c))
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
            } else {
                vec![s.to_string()]
            }
        })
        .filter(|t| !t.is_empty() && !is_stop_word(t))
        .collect();
    tokens
}

fn is_stop_char(c: char) -> bool {
    matches!(
        c,
        '。' | '，' | '！' | '？' | '、' | '；' | ':' | ';' | ',' | '.' | '!' | '?'
    )
}

fn is_stop_word(word: &str) -> bool {
    let stops: &[&str] = &[
        "的",
        "了",
        "是",
        "在",
        "我",
        "有",
        "和",
        "就",
        "不",
        "人",
        "都",
        "一",
        "一个",
        "上",
        "也",
        "很",
        "到",
        "说",
        "要",
        "去",
        "你",
        "会",
        "着",
        "没有",
        "看",
        "好",
        "自己",
        "这",
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "must",
        "shall",
        "can",
        "need",
        "dare",
        "ought",
        "used",
        "to",
        "of",
        "in",
        "for",
        "on",
        "with",
        "at",
        "by",
        "from",
        "as",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "under",
        "and",
        "but",
        "or",
        "yet",
        "so",
        "if",
        "because",
        "although",
        "though",
        "while",
        "where",
        "when",
        "that",
        "which",
        "who",
        "whom",
        "whose",
        "what",
        "whatever",
        "whoever",
        "whomever",
        "this",
        "these",
        "those",
        "i",
        "me",
        "my",
        "myself",
        "we",
        "our",
        "ours",
        "ourselves",
        "you",
        "your",
        "yours",
        "yourself",
        "yourselves",
        "he",
        "him",
        "his",
        "himself",
        "she",
        "her",
        "hers",
        "herself",
        "it",
        "its",
        "itself",
        "they",
        "them",
        "their",
        "theirs",
        "themselves",
    ];
    stops.contains(&word)
}

pub fn resolve_language(files: &HashMap<String, String>, requested: &str) -> String {
    if files.contains_key(requested) {
        return requested.to_string();
    }
    // 简化为 'zh' 或 'en'
    let short = if requested.starts_with("zh") {
        "zh"
    } else {
        "en"
    };
    if files.contains_key(short) {
        return short.to_string();
    }
    if files.contains_key("en") {
        return "en".to_string();
    }
    files
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| "en".to_string())
}

pub fn resolve_title(title: &GuideTitle, language: &str) -> String {
    if language.starts_with("zh") {
        title.zh.clone()
    } else {
        title.en.clone()
    }
}

fn load_guide_content(entry: &GuideIndexEntry, language: &str) -> Result<GuideContent, String> {
    let lang = resolve_language(&entry.files, language);
    let rel_path = format!("docs/guides/{}", entry.files.get(&lang).ok_or("No file")?);
    let path = resource_path(&rel_path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide {:?}: {}", path, e))?;

    // 文档较短时不截断；超长时截断至 4000 字节（覆盖所有现有帮助文档）
    const MAX_GUIDE_LEN: usize = 4000;
    let truncated = if content.len() > MAX_GUIDE_LEN {
        let mut end = MAX_GUIDE_LEN;
        while !content.is_char_boundary(end) {
            end -= 1;
        }
        let mut cut = &content[..end];
        if let Some(pos) = cut.rfind("\n\n") {
            cut = &content[..pos];
        } else if let Some(pos) = cut.rfind('\n') {
            cut = &content[..pos];
        }
        format!("{}\n\n（文档内容过长，已截断）", cut)
    } else {
        content
    };

    Ok(GuideContent {
        id: entry.id.clone(),
        title: resolve_title(&entry.title, language),
        content: truncated,
    })
}

pub fn find_relevant_guides_internal(
    query: &str,
    language: &str,
) -> Result<Vec<GuideContent>, String> {
    let index = load_guide_index()?;
    ensure_summary_cache(&index);
    let tokens = tokenize_query(query);
    if tokens.is_empty() {
        return Ok(vec![]);
    }

    // 意图分类：简单规则加权
    let is_howto = tokens
        .iter()
        .any(|t| ["怎么", "如何", "怎样", "how", "步骤", "step"].contains(&t.as_str()));
    let is_concept = tokens
        .iter()
        .any(|t| ["什么是", "为什么", "what", "why", "explain"].contains(&t.as_str()));

    let threshold = if tokens.len() >= 2 { 2 } else { 1 };

    let summary_cache = get_summary_cache();

    let mut scored: Vec<(GuideIndexEntry, i32)> = vec![];
    for guide in &index.guides {
        let mut score = 0;
        let title_text = resolve_title(&guide.title, language).to_lowercase();
        let summary_text = summary_cache
            .get(&guide.id)
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        for token in &tokens {
            if guide
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(token))
            {
                score += 1;
            }
            if title_text.contains(token) {
                score += 10;
            }
            if summary_text.contains(token) {
                score += 2; // 摘要命中权重介于关键词和标题之间
            }
        }
        // 意图加权
        if is_howto && guide.category == "objects" {
            score += 2;
        }
        if is_concept && guide.category == "security" {
            score += 2;
        }
        if score >= threshold {
            scored.push((guide.clone(), score));
        }
    }

    scored.sort_by_key(|b| std::cmp::Reverse(b.1));

    // Top-3（v2.0 从 Top-1 扩展为 3 篇互补）
    let mut results = vec![];
    for (entry, _) in scored.into_iter().take(3) {
        match load_guide_content(&entry, language) {
            Ok(g) => results.push(g),
            Err(e) => eprintln!("Guide load error: {}", e),
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn llm_find_guides(query: String, language: String) -> Result<Vec<GuideContent>, String> {
    find_relevant_guides_internal(&query, &language)
}

// =============================================================================
// Guide System Commands (§18)
// =============================================================================

#[tauri::command]
pub async fn guide_load_index() -> Result<GuideIndex, String> {
    load_guide_index()
}

#[tauri::command]
pub async fn guide_load_content(
    guide_id: String,
    language: String,
) -> Result<GuideContent, String> {
    let index = load_guide_index()?;
    let entry = index
        .guides
        .into_iter()
        .find(|g| g.id == guide_id)
        .ok_or_else(|| format!("Guide not found: {}", guide_id))?;
    let lang = resolve_language(&entry.files, &language);
    let rel_path = format!("docs/guides/{}", entry.files.get(&lang).ok_or("No file")?);
    let path = resource_path(&rel_path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide {:?}: {}", path, e))?;
    Ok(GuideContent {
        id: entry.id,
        title: resolve_title(&entry.title, &language),
        content,
    })
}

/// 加载全文搜索索引（缓存优先）
pub fn load_search_index_impl() -> Result<SearchIndex, String> {
    if let Some(idx) = get_search_index_cache() {
        return Ok(idx);
    }
    let path = resource_path("docs/guides/search-index.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read search index at {:?}: {}", path, e))?;
    let index: SearchIndex = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse search index: {}", e))?;
    set_search_index_cache(index.clone());
    Ok(index)
}

#[tauri::command]
pub async fn guide_search(query: String, language: String) -> Result<Vec<GuideContent>, String> {
    let index = load_guide_index()?;
    let search_index = load_search_index_impl()?;
    let tokens: Vec<String> = query
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if tokens.is_empty() {
        return Ok(vec![]);
    }

    let mut scored: Vec<(GuideIndexEntry, i32)> = vec![];
    for guide in index.guides {
        let mut score = 0;
        let title_text = resolve_title(&guide.title, &language).to_lowercase();
        for token in &tokens {
            // Keyword match (score 1)
            if guide
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(token))
            {
                score += 1;
            }
            // Title match (score 10) — heavily prioritised over content-only matches
            if title_text.contains(token) {
                score += 10;
            }
            // Full-text content match via pre-built search index (score 2)
            if let Some(guide_ids) = search_index.words.get(token) {
                if guide_ids.contains(&guide.id) {
                    score += 2;
                }
            }
        }
        if score >= 1 {
            scored.push((guide, score));
        }
    }

    scored.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut results = vec![];
    for (entry, _) in scored.into_iter().take(10) {
        match guide_load_content(entry.id.clone(), language.clone()).await {
            Ok(g) => results.push(g),
            Err(e) => eprintln!("Guide load error: {}", e),
        }
    }
    Ok(results)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    pub words: std::collections::HashMap<String, Vec<String>>,
    pub titles: std::collections::HashMap<String, GuideTitle>,
}

#[tauri::command]
pub async fn guide_load_search_index() -> Result<SearchIndex, String> {
    load_search_index_impl()
}
