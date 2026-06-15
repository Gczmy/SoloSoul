import type { TFunction } from 'i18next';
import type { CustomPage } from '@/stores/settingsStore';

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
