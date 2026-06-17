import type { OcrTierInfo } from '@/lib/ipc';
import type { TFunction } from 'i18next';

/**
 * 获取已本地化的 OCR 模型档位名称与描述。
 * 后端返回的 name/description 保留为 fallback，避免新增语言时缺失。
 */
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
