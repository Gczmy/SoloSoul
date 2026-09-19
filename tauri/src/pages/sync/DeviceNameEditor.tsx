import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pencil } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { formatPeerName } from '@/lib/syncPeer';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import type { SyncPeer } from '@/stores/syncStore';
import styles from './DeviceNameEditor.module.css';

export function DeviceNameEditor({
  peer,
  onSave,
  disabled,
}: {
  peer: SyncPeer;
  onSave: (peerId: string, name: string) => Promise<void>;
  disabled?: boolean;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  if (!editing)
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
        <span
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 600,
            overflowWrap: 'anywhere',
            minWidth: 0,
          }}
        >
          {formatPeerName(peer)}
        </span>
        <Button
          size="sm"
          variant="tertiary"
          disabled={disabled}
          aria-label={t('settings:sync_device_name_edit')}
          title={t('settings:sync_device_name_edit')}
          onClick={() => {
            setDraft(peer.customName ?? formatPeerName(peer));
            setError('');
            setEditing(true);
          }}
        >
          <Pencil size={16} />
        </Button>
      </div>
    );
  return (
    <form
      onSubmit={async (event) => {
        event.preventDefault();
        if (saving || disabled) return;
        setSaving(true);
        setError('');
        try {
          await onSave(peer.id, draft.trim());
          setEditing(false);
        } catch (err) {
          setError(resolveBackendErrorMessage(err));
        } finally {
          setSaving(false);
        }
      }}
      className={styles.form}
    >
      <div className={styles.field}>
        <Input
          aria-label={t('settings:sync_device_name_label')}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          maxLength={64}
          autoFocus
          disabled={saving || disabled}
          error={error}
        />
      </div>
      <span className={styles.hint}>{t('settings:sync_device_name_hint')}</span>
      <div className={styles.actions}>
        <Button
          type="button"
          size="sm"
          variant="secondary"
          disabled={saving}
          onClick={() => setEditing(false)}
        >
          {t('common:cancel')}
        </Button>
        <Button
          type="submit"
          size="sm"
          variant="primary"
          disabled={saving || disabled}
          loading={saving}
        >
          {t('common:save')}
        </Button>
      </div>
    </form>
  );
}
