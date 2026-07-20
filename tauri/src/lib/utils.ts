import i18next from '@/lib/i18n';
import type { TFunction } from 'i18next';
import type { OcrTierInfo } from '@/lib/ipc';
import type { CustomPage } from '@/stores/settingsStore';

// ============================================================================
// From format.ts
// ============================================================================

const formatBytesNumberFormatter = new Intl.NumberFormat(undefined, {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});

/** 将字节数格式化为人类可读字符串（B / KB / MB / GB）。
 *  使用 1024 进制（二进制），通过 Intl.NumberFormat 实现本地化数字格式。 */
export function formatBytes(bytes: number): string {
  if (bytes < 0) return `0 ${i18next.t('common:byte_unit', 'B')}`;
  if (bytes < 1024) return `${bytes} ${i18next.t('common:byte_unit', 'B')}`;
  if (bytes < 1024 * 1024)
    return `${formatBytesNumberFormatter.format(bytes / 1024)} ${i18next.t('common:byte_unit_kb', 'KB')}`;
  if (bytes < 1024 * 1024 * 1024)
    return `${formatBytesNumberFormatter.format(bytes / (1024 * 1024))} ${i18next.t('common:byte_unit_mb', 'MB')}`;
  return `${formatBytesNumberFormatter.format(bytes / (1024 * 1024 * 1024))} ${i18next.t('common:byte_unit_gb', 'GB')}`;
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

// ============================================================================
// Shared backend error prefix resolution (Phase 2.4)
// ============================================================================

/** 解析后的后端错误前缀信息。 */
export interface ResolvedErrorPrefix {
  kind: string;
  code: string;
  payload: string | null;
}

const ERROR_PREFIXES: Array<{ prefix: string; kind: string }> = [
  { prefix: '__EXPORT_ERR__:', kind: 'export' },
  { prefix: '__IMPORT_ERR__:', kind: 'import' },
  { prefix: '__BIO_ERR__:', kind: 'biometric' },
];

/**
 * 解析后端错误消息中已知的前缀，提取 kind / code / payload。
 * 若未匹配任何已知前缀，返回 null。
 */
export function resolveI18nPrefix(message: string): ResolvedErrorPrefix | null {
  for (const { prefix, kind } of ERROR_PREFIXES) {
    const rest = tryParsePrefixedError(message, prefix);
    if (rest !== null) {
      const sep = rest.indexOf(':');
      const code = sep >= 0 ? rest.slice(0, sep) : rest;
      const payload = sep >= 0 ? rest.slice(sep + 1) : null;
      return { kind, code, payload };
    }
  }
  return null;
}
