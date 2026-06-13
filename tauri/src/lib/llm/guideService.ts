// =============================================================================
// 帮助文档前端服务 (RAG 向量检索)
// =============================================================================
// 通过 IPC 调用 Rust 后端进行向量相似度检索，获取 Top-K 文档片段。
// Fallback 到关键词检索当 embedding 不可用时。
// =============================================================================

import { invoke } from '@tauri-apps/api/core';

export interface GuideContent {
  id: string;
  title: string;
  content: string;
}

export interface GuideChunk {
  guideId: string;
  guideTitle: string;
  chunkText: string;
  similarity: number;
}

/**
 * 向量检索：获取与用户查询最相关的文档片段（Top-K）。
 * 后端自动 fallback 到关键词检索当 embedding 不可用时。
 * @param query 用户查询文本
 * @param language 当前界面语言
 * @param topK 返回片段数量（默认 3）
 */
export async function searchGuideChunks(
  query: string,
  language: string,
  topK = 3,
): Promise<GuideChunk[]> {
  try {
    const chunks = await invoke<GuideChunk[]>('llm_search_guide_chunks', {
      query,
      language,
      topK,
    });
    return chunks;
  } catch {
    return [];
  }
}

/**
 * 将检索到的文档片段格式化为 system message 注入文本。
 * @param chunks 文档片段列表
 * @returns 格式化后的注入文本
 */
export function formatChunksAsSystemMessage(chunks: GuideChunk[]): string | null {
  if (chunks.length === 0) return null;

  const parts: string[] = ['以下是与用户问题相关的官方功能使用文档片段，请优先依据这些片段回答：'];

  chunks.forEach((chunk, i) => {
    parts.push(
      `\n【文档片段 ${i + 1}】来源：《${chunk.guideTitle}》 相关度：${(chunk.similarity * 100).toFixed(1)}%`,
      '```text',
      chunk.chunkText,
      '```',
    );
  });

  parts.push(
    '\n如果以上文档片段中完全没有涉及用户问题的内容，才回答"我暂时不清楚这个细节，建议你查看软件内的帮助页面"。否则请基于文档积极回答。',
  );

  return parts.join('\n');
}

// ── 遗留关键词检索（作为 fallback 保留）─────────────────────────

/**
 * 检索与用户查询相关的帮助文档（关键词匹配，legacy）。
 * @param query 用户查询文本
 * @param language 当前界面语言
 * @returns 匹配的指南内容，无匹配时返回 null
 */
export async function findRelevantGuides(
  query: string,
  language: string,
): Promise<GuideContent | null> {
  try {
    const guides = await invoke<GuideContent[]>('llm_find_guides', {
      query,
      language,
    });
    if (guides.length === 0) return null;
    return guides[0];
  } catch {
    return null;
  }
}

/**
 * 将匹配到的指南内容包装为 system message 格式（legacy）。
 * @param guide 指南内容
 * @returns 格式化后的注入文本，guide 为 null 时返回 null
 */
export function formatGuideAsSystemMessage(guide: GuideContent | null): string | null {
  if (!guide) return null;
  return `---
以下是与用户问题相关的官方功能使用文档。优先依据文档中的信息回答，允许用自然语言转述文档内容，但禁止编造文档中不存在的信息（如快捷键、菜单路径、不存在的功能按钮等）。

【文档：${guide.title}】
${guide.content}
【文档结束】

如果文档中完全没有涉及用户问题的内容，才回答"我暂时不清楚这个细节，建议你查看软件内的帮助页面"。否则请基于文档积极回答。
---`;
}
