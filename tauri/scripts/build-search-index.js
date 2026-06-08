/**
 * 构建时搜索索引预生成脚本
 * 读取 docs/guides/ 下的所有指南内容，生成倒排索引。
 *
 * 用法：node scripts/build-search-index.js
 * 输出：src-tauri/resources/docs/guides/search-index.json
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const GUIDES_DIR = path.join(__dirname, '../src-tauri/resources/docs/guides');
const INDEX_PATH = path.join(GUIDES_DIR, 'index.json');
const OUTPUT_PATH = path.join(GUIDES_DIR, 'search-index.json');

// 停用词
const STOP_WORDS = new Set([
  '的', '了', '是', '在', '我', '有', '和', '就', '不', '人', '都', '一', '上', '也', '很', '到', '说', '要', '去', '你', '会', '着', '没有', '看', '好', '这', '个', '为', '之', '与', '及', '等',
  'the', 'a', 'an', 'is', 'are', 'was', 'were', 'be', 'been', 'being', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could', 'should', 'may', 'might', 'must', 'to', 'of', 'in', 'for', 'on', 'with', 'at', 'by', 'from', 'as', 'and', 'but', 'or', 'if', 'it', 'its',
]);

/** 从文本中提取有意义的 token */
function extractTokens(text) {
  const tokens = new Set();
  // 移除 Markdown 标记
  const clean = text
    .replace(/[#*`\[\]!|>\-_]/g, ' ')
    .replace(/\n+/g, ' ');

  const parts = clean.split(/\s+|[^\w\u4e00-\u9fff]+/);
  for (const part of parts) {
    const lower = part.toLowerCase().trim();
    if (lower.length < 2) continue;
    if (STOP_WORDS.has(lower)) continue;
    tokens.add(lower);

    // 中文：同时添加单字 token（用于前缀匹配）
    if (/[\u4e00-\u9fff]/.test(lower)) {
      for (const ch of lower) {
        if (!STOP_WORDS.has(ch)) {
          tokens.add(ch);
        }
      }
    }
  }
  return Array.from(tokens);
}

function main() {
  const indexData = JSON.parse(fs.readFileSync(INDEX_PATH, 'utf-8'));
  const wordIndex = {}; // word -> Set<guideId>
  const titles = {};    // guideId -> { zh, en }

  for (const guide of indexData.guides) {
    titles[guide.id] = guide.title;

    // 索引标题
    for (const lang of ['zh', 'en']) {
      const titleText = guide.title[lang] || '';
      for (const token of extractTokens(titleText)) {
        if (!wordIndex[token]) wordIndex[token] = new Set();
        wordIndex[token].add(guide.id);
      }
    }

    // 索引关键词
    for (const kw of guide.keywords) {
      const lower = kw.toLowerCase().trim();
      if (lower.length < 2) continue;
      if (!wordIndex[lower]) wordIndex[lower] = new Set();
      wordIndex[lower].add(guide.id);
    }

    // 索引内容（所有语言版本）
    for (const [lang, fileName] of Object.entries(guide.files)) {
      const filePath = path.join(GUIDES_DIR, fileName);
      if (fs.existsSync(filePath)) {
        const content = fs.readFileSync(filePath, 'utf-8');
        for (const token of extractTokens(content)) {
          if (!wordIndex[token]) wordIndex[token] = new Set();
          wordIndex[token].add(guide.id);
        }
      }
    }
  }

  // 转换为数组格式
  const output = {
    words: {},
    titles,
  };
  for (const [word, guideIds] of Object.entries(wordIndex)) {
    output.words[word] = Array.from(guideIds).sort();
  }

  fs.writeFileSync(OUTPUT_PATH, JSON.stringify(output, null, 2), 'utf-8');
  console.log(`Search index built: ${Object.keys(output.words).length} words, ${indexData.guides.length} guides`);
}

main();
