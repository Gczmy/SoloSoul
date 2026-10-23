import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  MoreHorizontal,
  Clock,
  Paperclip,
  Pencil,
  Trash2,
  RefreshCw,
  Lock,
  LockOpen,
  type LucideIcon,
} from 'lucide-react';
import { AndroidSheet } from './AndroidSheet';
import { ObjectEditLockBadge } from '@/components/object/ObjectEditLockBadge';
import { useObjectEditLocked } from '@/hooks/useObjectEditLocked';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { useObjectStore, type ObjectData, type ObjectSummary } from '@/stores/objectStore';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { createSessionRequests } from '@/lib/sessionRequests';

export function AndroidObjectRow({
  obj,
  Icon,
  collectionLabel,
  templateName,
  sensitivities,
  snapshotCount,
  attachmentCount,
  needsSync,
  onClick,
  onHistory,
  onAttachments,
  onEdit,
  onDelete,
  onSync,
  onDismissSync,
}: {
  obj: ObjectSummary | ObjectData;
  Icon: LucideIcon;
  collectionLabel: string;
  templateName: string;
  sensitivities: SensitivityLevel[];
  snapshotCount?: number;
  attachmentCount?: number;
  needsSync: boolean;
  onClick: () => void;
  onHistory: () => void;
  onAttachments: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onSync?: () => void;
  onDismissSync?: () => void;
}) {
  const { t } = useTranslation(['common', 'editor']);
  const editLocked = useObjectEditLocked(obj.id, obj.properties);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const setObjectLocked = useObjectStore((s) => s.setObjectLocked);
  const { onError, onSuccess } = useToastError();
  const [requests] = useState(createSessionRequests);
  useEffect(() => () => requests.invalidate(), [requests]);
  const context = JSON.stringify([accountId, obj.id]);
  const currentContext = useRef(context);
  currentContext.current = context;
  const pendingContext = useRef<string | null>(null);
  const [lockPendingContext, setLockPendingContext] = useState<string | null>(null);
  const lockPending = lockPendingContext === context;
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const action = (callback: () => void) => {
    setOpen(false);
    callback();
  };
  const toggleLock = async () => {
    if (pendingContext.current === context) return;
    const request = requests.begin();
    const isCurrent = () => request.isCurrent() && currentContext.current === context;
    const locked = !editLocked;
    pendingContext.current = context;
    setLockPendingContext(context);
    try {
      await setObjectLocked(obj.id, locked);
      if (!isCurrent()) return;
      setOpen(false);
      onSuccess(t(locked ? 'object_locked_success' : 'object_unlocked_success'));
    } catch (error) {
      if (isCurrent()) onError(error, t('object_lock_failed'));
    } finally {
      if (pendingContext.current === context) pendingContext.current = null;
      if (isCurrent()) setLockPendingContext(null);
    }
  };
  return (
    <div className="android-object-row" data-testid="workspace-object-card">
      <button type="button" className="android-row-button" onClick={onClick} disabled={lockPending}>
        <span className="android-row-icon">
          <Icon size={24} />
        </span>
        <span className="android-row-copy">
          <strong>{obj.name}</strong>
          <small>
            {collectionLabel} · {templateName}
          </small>
          <span className="android-row-meta">
            <ObjectEditLockBadge objectId={obj.id} properties={obj.properties} />
            {sensitivities.map((level) => (
              <SensitivityBadge key={level} level={level} showText={false} />
            ))}
            {needsSync && <RefreshCw size={16} aria-label={t('editor:template_updated_hint')} />}
          </span>
        </span>
      </button>
      <button
        ref={trigger}
        type="button"
        className="android-icon-button"
        aria-label={t('material.object_actions', { name: obj.name })}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen(true)}
      >
        <MoreHorizontal size={24} />
      </button>
      {open && (
        <AndroidSheet title={obj.name} onClose={() => setOpen(false)} trigger={trigger.current}>
          <div className="android-create-options" aria-busy={lockPending}>
            <button type="button" disabled={lockPending} onClick={() => action(onEdit)}>
              <Pencil />
              <strong>{t('edit')}</strong>
            </button>
            <button type="button" disabled={lockPending} onClick={() => void toggleLock()}>
              {editLocked ? <LockOpen /> : <Lock />}
              <strong>{t(editLocked ? 'object_unlock' : 'object_lock')}</strong>
            </button>
            <button type="button" onClick={() => action(onHistory)}>
              <Clock />
              <strong>
                {t('material.history')}
                {snapshotCount !== undefined ? ` · ${snapshotCount}` : ''}
              </strong>
            </button>
            <button type="button" onClick={() => action(onAttachments)}>
              <Paperclip />
              <strong>
                {t('material.attachments')}
                {attachmentCount !== undefined ? ` · ${attachmentCount}` : ''}
              </strong>
            </button>
            {needsSync && onSync && !editLocked && (
              <button type="button" disabled={lockPending} onClick={() => action(onSync)}>
                <RefreshCw />
                <strong>{t('editor:template_updated_hint')}</strong>
              </button>
            )}
            {needsSync && onDismissSync && !editLocked && (
              <button type="button" disabled={lockPending} onClick={() => action(onDismissSync)}>
                <strong>{t('material.skip_sync')}</strong>
              </button>
            )}
            <button
              type="button"
              disabled={editLocked || lockPending}
              onClick={() => action(onDelete)}
            >
              <Trash2 />
              <strong>{t('delete')}</strong>
            </button>
          </div>
        </AndroidSheet>
      )}
    </div>
  );
}
