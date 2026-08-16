// =============================================================================
// 帮助文档前端服务 (RAG 向量检索)
// =============================================================================
// 通过 IPC 调用 Rust 后端进行向量相似度检索，获取 Top-K 文档片段。
// Fallback 到关键词检索当 embedding 不可用时。
// =============================================================================

import { invokeCommand as invoke } from '@/lib/ipcClient';

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
      topK: topK,
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
