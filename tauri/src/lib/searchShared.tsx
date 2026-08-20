// 搜索共享逻辑（P141）——SearchPage 与 SearchPopover 共用。
// 收敛两处逐字重复的：SearchItem 类型、Highlight 高亮组件、敏感级别排序、
// 页面名/图标解析、系统页面翻译匹配、匹配字段提示渲染、缓存键与请求体构造。

import type { TFunction } from 'i18next';
import type { LucideIcon } from 'lucide-react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { searchCache } from '@/lib/searchCache';
import type { CustomPage } from '@/stores/settingsStore';
import { SensitivityLevel } from '@/components/ui/SensitivityBadge';

/** 系统页面 key（与 FILTER_PAGES 同源，SearchPage/SearchPopover 共用）。 */
export const SYSTEM_PAGE_KEYS = [
  'identity',
  'travel',
  'financial',
  'professional',
  'document',
] as const;

/** 统一搜索结果项结构。 */
export interface SearchItem {
  objectId: string;
  name: string;
  typeId: string;
  itemType?: string;
  parentId?: string;
  templateName?: string;
  templateDeleted?: boolean;
  /** 对象所属模板的图标 ID——解析「对象自身图标」用（与 workspace 对象卡片一致）。 */
  templateIconId?: string;
  objectCount?: number;
  fieldCount?: number;
  matchedField?: string;
  matchedValue?: string;
  matchType?: 'fieldName' | 'fieldValue' | 'name' | 'template';
  sensitivityLevels?: string[];
  relevance: number;
}

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

/** 将敏感级别数组按 public→critical 升序排序。 */
export function sortSensitivityLevels(levels: string[]): SensitivityLevel[] {
  return levels
    .filter((lvl): lvl is SensitivityLevel => SENSITIVITY_ORDER.includes(lvl as SensitivityLevel))
    .sort((a, b) => SENSITIVITY_ORDER.indexOf(a) - SENSITIVITY_ORDER.indexOf(b));
}

/** 将文本分段并对命中关键词加粗（大小写不敏感）。 */
export function Highlight({ text, query }: { text: string; query: string }) {
  if (!query.trim()) return <>{text}</>;
  const lowerQuery = query.toLowerCase();
  const lowerText = text.toLowerCase();
  const parts: React.ReactNode[] = [];
  let i = 0;
  while (i < text.length) {
    const idx = lowerText.indexOf(lowerQuery, i);
    if (idx === -1) {
      parts.push(text.slice(i));
      break;
    }
    if (idx > i) parts.push(text.slice(i, idx));
    parts.push(
      <mark
        key={`${idx}-${query}`}
        style={{
          fontWeight: 700,
          color: 'var(--accent-primary)',
          background: 'transparent',
        }}
      >
        {text.slice(idx, idx + query.length)}
      </mark>,
    );
    i = idx + query.length;
  }
  return <>{parts}</>;
}

/** 检测查询词是否命中翻译后的系统页面名，命中返回英文 key。 */
export function matchPageTranslation(query: string, t: TFunction): string | null {
  const q = query.toLowerCase().trim();
  for (const key of SYSTEM_PAGE_KEYS) {
    const label = t(`navigation:${key}`).toLowerCase();
    if (label === q || label.includes(q) || q.includes(label)) {
      return key;
    }
  }
  return null;
}

/** 解析搜索结果的展示名：系统页面翻译、自定义页面/对象用存储名。 */
export function resolveResultName(
  item: { itemType?: string; objectId: string; name: string },
  customPages: CustomPage[],
  t: TFunction,
): string {
  if (item.itemType === 'page') {
    const systemKey = (SYSTEM_PAGE_KEYS as readonly string[]).includes(item.objectId)
      ? item.objectId
      : null;
    if (systemKey) {
      return t(`navigation:${systemKey}`);
    }
    const cp = customPages.find((p) => p.id === item.objectId);
    if (cp) return cp.name;
  }
  return item.name;
}

/** 解析搜索结果的图标：系统页面按 objectId、对象按模板图标、自定义页面按 iconId。 */
export function resolveResultIcon(
  item: {
    itemType?: string;
    typeId: string;
    objectId: string;
    templateIconId?: string;
  },
  customPages: CustomPage[],
): LucideIcon {
  if (item.itemType === 'page') {
    if (item.objectId in PAGE_ICON_MAP) {
      return PAGE_ICON_MAP[item.objectId as keyof typeof PAGE_ICON_MAP];
    }
    const cp = customPages.find((p) => p.id === item.objectId);
    if (cp) {
      return resolveCustomIcon(cp.iconId);
    }
    return PAGE_ICON_MAP.custom;
  }
  // 对象优先显示所属模板的图标（与 workspace 对象卡片同源）；缺失时回退到所属页面图标。
  if (item.templateIconId) {
    return resolveCustomIcon(item.templateIconId);
  }
  if (item.typeId in PAGE_ICON_MAP) {
    return PAGE_ICON_MAP[item.typeId as keyof typeof PAGE_ICON_MAP];
  }
  const cp = customPages.find((p) => p.id === item.typeId);
  if (cp) {
    return resolveCustomIcon(cp.iconId);
  }
  return PAGE_ICON_MAP.custom;
}

/** 解析字段路径的最后一段为可读标签（i18n 优先，回退原文）。 */
function resolveFieldLabel(fieldPath: string | undefined, t: TFunction): string {
  if (!fieldPath) return '';
  const lastSegment = fieldPath.split('.').pop() || fieldPath;
  return t(`editor:fields.${lastSegment}`, lastSegment);
}

/** 渲染搜索结果的字段命中提示（字段名/字段值/模板命中）。 */
export function MatchHint({
  item,
  query,
  t,
}: {
  item: SearchItem;
  query: string;
  t: TFunction;
}) {
  if (!item.matchedField || item.itemType === 'page' || item.matchType === 'name') return null;
  const fieldLabel = resolveFieldLabel(item.matchedField, t);
  if (item.matchType === 'fieldName' && item.matchedValue) {
    return (
      <span>
        {' · '}
        {t('settings:search_field_label', '字段名')}：
        <Highlight text={fieldLabel} query={query} />
      </span>
    );
  }
  if (item.matchType === 'fieldValue' && item.matchedValue) {
    return (
      <span>
        {' · '}
        <Highlight text={fieldLabel} query={query} />
        {': '}
        <Highlight text={item.matchedValue} query={query} />
      </span>
    );
  }
  if (item.matchType === 'template' && item.matchedValue) {
    return (
      <span>
        {' · '}
        {t('settings:search_type_template')}：<Highlight text={item.matchedValue} query={query} />
      </span>
    );
  }
  return null;
}

/** 统一搜索结果缓存键参数。pageKey 优先级高于 filter 的系统页；自定义页走 parentId。 */
export function buildSearchCacheParams(
  accountId: string,
  query: string,
  pageKey: string | null,
  filter: string | null,
  customPages: CustomPage[],
) {
  const isCustom = filter ? customPages.some((p) => p.id === filter) : false;
  const effectiveCollectionType = pageKey ?? (filter && !isCustom ? filter : null);
  const parentId = filter && isCustom ? filter : null;
  return { cacheKey: searchCache.buildKey(accountId, query, effectiveCollectionType, parentId), effectiveCollectionType, parentId };
}

/** 构造 search_unified 请求体。 */
export function buildSearchPayload(
  accountId: string,
  query: string,
  pageKey: string | null,
  filter: string | null,
  customPages: CustomPage[],
): Record<string, unknown> {
  const payload: Record<string, unknown> = { accountId, query, limit: 50 };
  if (pageKey) {
    payload.typeId = pageKey;
  }
  if (filter) {
    const isCustom = customPages.some((p) => p.id === filter);
    if (isCustom) {
      payload.parentId = filter;
    } else {
      payload.typeId = filter;
    }
  }
  return payload;
}

/** 系统页面搜索时确保结果含该页面项（后端可能不返回），置顶一个合成 page 结果。 */
export function ensurePageResultExists(items: SearchItem[], pageKey: string): SearchItem[] {
  const pageExists = items.some((i) => i.itemType === 'page' && i.objectId === pageKey);
  if (pageExists) {
    return items;
  }
  return [
    {
      objectId: pageKey,
      name: pageKey,
      typeId: pageKey,
      itemType: 'page',
      objectCount: undefined,
      matchedField: undefined,
      matchedValue: undefined,
      matchType: undefined,
      sensitivityLevels: undefined,
      relevance: 99,
    } as SearchItem,
    ...items,
  ];
}

/**
 * P018: 统一搜索执行（SearchPage / SearchPopover 的 doSearch 收敛）。
 *
 * 共享：空查询守卫、页面名翻译匹配、缓存命中短路、search_unified 调用、
 * 页面结果补齐、缓存写入与错误处理。差异仅剩调用方的 state setter 与 filter。
 */
export async function runUnifiedSearch(params: {
  accountId: string | null | undefined;
  query: string;
  filter: string | null;
  customPages: CustomPage[];
  t: TFunction;
  onError: (e: unknown, fallback: string) => void;
  setResults: (items: SearchItem[]) => void;
  setHasSearched: (v: boolean) => void;
  setIsSearching: (v: boolean) => void;
}): Promise<void> {
  const { accountId, query, filter, customPages, t, onError } = params;
  const { setResults, setHasSearched, setIsSearching } = params;

  if (!accountId || (!query.trim() && !filter)) {
    setResults([]);
    setHasSearched(false);
    return;
  }

  // filter 存在时不参与页面名翻译匹配（popover 现状；page 页 filter 恒为 null 等价）
  const pageKey = !filter ? matchPageTranslation(query, t) : null;
  const { cacheKey } = buildSearchCacheParams(accountId, query, pageKey, filter, customPages);
  const cached = searchCache.get<SearchItem[]>(cacheKey);
  if (cached) {
    setResults(cached);
    setHasSearched(true);
    return;
  }

  setIsSearching(true);
  setHasSearched(true);
  try {
    const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
      'search_unified',
      buildSearchPayload(accountId, query, pageKey, filter, customPages),
    );
    const items = pageKey ? ensurePageResultExists(res.items, pageKey) : res.items;
    searchCache.set(cacheKey, items);
    setResults(items);
  } catch (e) {
    onError(e, t('common:search_failed'));
  } finally {
    setIsSearching(false);
  }
}
