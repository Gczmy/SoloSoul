import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { objectEditLocked } from '@/lib/objectEditLock';

/** 各入口共享已确认的锁状态，不触发额外读取或清空可见内容。 */
export function useObjectEditLocked(
  objectId?: string | null,
  properties?: Record<string, unknown> | null,
  fallback = false,
): boolean {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const cached = useObjectStore((s) => (objectId ? s.currentObjectCache[objectId] : undefined));
  const summaryLocked = useObjectStore((s) => {
    const summary = s.objects.find((object) => object.id === objectId);
    return summary?.properties ? objectEditLocked(summary.properties) : undefined;
  });
  return cached && cached.accountId === accountId
    ? objectEditLocked(cached.properties)
    : properties
      ? objectEditLocked(properties)
      : (summaryLocked ?? fallback);
}
