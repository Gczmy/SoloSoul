import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { createSessionRequests } from '@/lib/sessionRequests';
import { useToastError } from './useToastError';

/** 列表和详情菜单共用写入、重复提交保护及过期回执处理。 */
export function useObjectLockAction(objectId: string, editLocked: boolean) {
  const { t } = useTranslation('common');
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const setObjectLocked = useObjectStore((s) => s.setObjectLocked);
  const { onError, onSuccess } = useToastError();
  const [requests] = useState(createSessionRequests);
  useEffect(() => () => requests.invalidate(), [requests]);
  const context = JSON.stringify([accountId, objectId]);
  const currentContext = useRef(context);
  currentContext.current = context;
  const pendingContext = useRef<string | null>(null);
  const [lockPendingContext, setLockPendingContext] = useState<string | null>(null);
  const lockPending = lockPendingContext === context;

  const toggleLock = async (): Promise<boolean> => {
    if (pendingContext.current === context) return false;
    const request = requests.begin('lock');
    const isCurrent = () => request.isCurrent() && currentContext.current === context;
    const locked = !editLocked;
    pendingContext.current = context;
    setLockPendingContext(context);
    try {
      await setObjectLocked(objectId, locked);
      if (!isCurrent()) return false;
      onSuccess(t(locked ? 'object_locked_success' : 'object_unlocked_success'));
      return true;
    } catch (error) {
      if (isCurrent()) onError(error, t('object_lock_failed'));
      return false;
    } finally {
      if (pendingContext.current === context) pendingContext.current = null;
      if (isCurrent()) setLockPendingContext(null);
    }
  };

  return { lockPending, toggleLock };
}
