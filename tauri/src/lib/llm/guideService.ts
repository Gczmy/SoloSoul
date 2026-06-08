// =============================================================================
// 帮助文档前端服务
// =============================================================================
// 通过 IPC 调用 Rust 后端检索与用户查询相关的帮助文档。
// 文档 §7 规范：关键词匹配、动态阈值、Top-1、多语言回退。
// =============================================================================

import { invoke } from '@tauri-apps/api/core';

export interface GuideContent {
  id: string;
  title: string;
  content: string;
}

/**
 * 检索与用户查询相关的帮助文档。
 * @param query 用户查询文本
 * @param language 当前界面语言（如 'zh-CN'、'en-US'）
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
 * 将匹配到的指南内容包装为 system message 格式。
 * @param guide 指南内容
 * @returns 格式化后的注入文本，guide 为 null 时返回 null
 */
export function formatGuideAsSystemMessage(guide: GuideContent | null): string | null {
  if (!guide) return null;
  return `---
以下是与用户问题相关的功能使用文档，请参考这些信息回答用户问题。

【文档：${guide.title}】
${guide.content}
【文档结束】
---`;
}
