// =============================================================================
// 帮助文档前端 API
// =============================================================================
// 通过 IPC 调用 Rust 后端加载指南索引和内容。
// 纯本地调用，无网络依赖。
// =============================================================================

import { invoke } from '@tauri-apps/api/core';

/** 检测当前是否在 Tauri 运行环境中 */
function isTauriEnv(): boolean {
  if (typeof window === 'undefined') return false;
  return '__TAURI__' in window || '__TAURI_INTERNALS__' in window;
}

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

/** 包装 IPC 调用，增加超时保护（避免后端未启动时永久挂起） */
function invokeWithTimeout<T>(
  cmd: string,
  args?: Record<string, unknown>,
  timeoutMs = 15000,
): Promise<T> {
  if (!isTauriEnv()) {
    return Promise.reject(
      new Error(
        '当前不在 Tauri 环境中。帮助文档需要本地 Tauri 后端，请使用 npm run tauri dev 启动。',
      ),
    );
  }
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<T>((_, reject) => {
      setTimeout(() => {
        reject(
          new Error(
            `IPC 调用超时（${timeoutMs}ms）。请确认 Tauri 后端已启动（npm run tauri dev）。`,
          ),
        );
      }, timeoutMs);
    }),
  ]);
}

/** 加载指南索引 */
export async function loadGuideIndex(): Promise<GuideIndex> {
  return invokeWithTimeout<GuideIndex>('guide_load_index');
}

/** 加载单篇指南内容（通过后端读取文件） */
export async function loadGuideContent(guideId: string, language: string): Promise<GuideContent> {
  return invokeWithTimeout<GuideContent>('guide_load_content', { guideId, language });
}

/** 搜索指南（前端本地执行，基于索引关键词） */
export async function searchGuides(query: string, language: string): Promise<GuideContent[]> {
  return invokeWithTimeout<GuideContent[]>('guide_search', { query, language });
}
