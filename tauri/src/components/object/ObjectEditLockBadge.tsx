import { Lock, LockOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useObjectEditLocked } from '@/hooks/useObjectEditLocked';
import styles from './ObjectEditLockBadge.module.css';

export function ObjectEditLockBadge({
  objectId,
  properties,
  locked = false,
  compact = false,
  lockedOnly = false,
}: {
  objectId: string;
  properties?: Record<string, unknown>;
  locked?: boolean;
  compact?: boolean;
  /** 对象导航行只显示锁定状态，其他入口默认保留两种状态。 */
  lockedOnly?: boolean;
}) {
  const { t } = useTranslation('common');
  const isLocked = useObjectEditLocked(objectId, properties, locked);
  if (lockedOnly && !isLocked) return null;
  const label = t(isLocked ? 'object_locked' : 'object_editable');
  const Icon = isLocked ? Lock : LockOpen;
  return (
    <span
      className={styles.badge}
      data-object-edit-lock={isLocked ? 'locked' : 'editable'}
      title={label}
      aria-label={label}
    >
      <Icon size={13} aria-hidden="true" />
      {!compact && <span>{label}</span>}
    </span>
  );
}
