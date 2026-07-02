import type { TFunction } from 'i18next';
import type { OcrTierInfo } from '@/lib/ipc';
import type { CustomPage } from '@/stores/settingsStore';

// ============================================================================
// From format.ts
// ============================================================================

/** 将字节数格式化为人类可读字符串（B / KB / MB / GB）。 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/** 从字符串中提取前缀后的内容。 */
export function tryParsePrefixedError(message: string, prefix: string): string | null {
  const idx = message.indexOf(prefix);
  if (idx === -1) return null;
  return message.slice(idx + prefix.length).trim();
}

// ============================================================================
// From env.ts
// ============================================================================

/** 判断当前是否处于开发或调试模式。发布版本返回 false。 */
export function isDevOrDebug(): boolean {
  return (
    import.meta.env.DEV === true ||
    import.meta.env.MODE === 'debug' ||
    import.meta.env.VITE_SOLOSOUL_DEBUG === 'true'
  );
}

// ============================================================================
// From ocr.ts
// ============================================================================

/** 获取已本地化的 OCR 模型档位名称与描述。 */
export function getTierLabel(
  t: TFunction,
  tier: OcrTierInfo,
): { name: string; description: string } {
  const key = tier.tier.toLowerCase();
  const nameKey = `tier_${key}_name` as const;
  const descKey = `tier_${key}_description` as const;
  return {
    name: t(`ocr:${nameKey}`, { defaultValue: tier.name }),
    description: t(`ocr:${descKey}`, { defaultValue: tier.description }),
  };
}

// ============================================================================
// From pageLabels.ts
// ============================================================================

const BUILTIN_COLLECTIONS = ['identity', 'travel', 'financial', 'professional'] as const;

export function resolveCollectionLabel(
  collectionType: string,
  customPages: CustomPage[],
  t: TFunction,
): string {
  if (BUILTIN_COLLECTIONS.includes(collectionType as (typeof BUILTIN_COLLECTIONS)[number])) {
    return t(`navigation:${collectionType}`);
  }
  return customPages.find((p) => p.id === collectionType)?.name || collectionType;
}
