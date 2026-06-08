// =============================================================================
// 帮助文档前端 API
// =============================================================================
// 通过 IPC 调用 Rust 后端加载指南索引和内容。
// 纯本地调用，无网络依赖。
// =============================================================================

import { invoke } from '@tauri-apps/api/core';

export interface GuideTitle {
  zh: string;
  en: string;
}

export interface GuideCategoryMeta {
  id: string;
  title: GuideTitle;
  order: number;
}

export interface GuideIndexEntry {
  id: string;
  title: GuideTitle;
  category: string;
  order: number;
  keywords: string[];
  files: Record<string, string>;
}

export interface GuideIndex {
  guides: GuideIndexEntry[];
  categories: GuideCategoryMeta[];
}

export interface GuideContent {
  id: string;
  title: string;
  content: string;
}

/** 加载指南索引 */
export async function loadGuideIndex(): Promise<GuideIndex> {
  return invoke<GuideIndex>('guide_load_index');
}

/** 加载单篇指南内容（通过后端读取文件） */
export async function loadGuideContent(guideId: string, language: string): Promise<GuideContent> {
  return invoke<GuideContent>('guide_load_content', { guideId, language });
}

/** 搜索指南（前端本地执行，基于索引关键词） */
export async function searchGuides(query: string, language: string): Promise<GuideContent[]> {
  return invoke<GuideContent[]>('guide_search', { query, language });
}
