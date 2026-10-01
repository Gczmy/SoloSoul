// =============================================================================
// 帮助文档前端服务 (RAG 向量检索)
// =============================================================================
// 通过 IPC 调用 Rust 后端进行向量相似度检索，获取 Top-K 文档片段。
// Fallback 到关键词检索当 embedding 不可用时。
// =============================================================================

import { invokeCommand as invoke } from '@/lib/ipcClient';

// 发送上下文使用生成的 Input DTO；RAG 查询命令迁移由 RF-908 承接。
export type { GuideChunkInput as GuideChunk } from '@/lib/generated/ipcContracts';
import type { GuideChunkInput as GuideChunk } from '@/lib/generated/ipcContracts';

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
