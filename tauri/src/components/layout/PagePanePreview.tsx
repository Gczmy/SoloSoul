import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowRight } from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { toObjectDataView, type ObjectDataView } from '@/lib/objectViewModel';
import { ObjectRulerPreview } from '@/components/workspace/ObjectRulerPreview';
import styles from '@/components/workspace/WorkspaceObjectRuler.module.css';

export function PagePanePagePreview({
  name,
  description,
  count,
  onNavigate,
}: {
  name: string;
  description?: string;
  count: number;
  onNavigate: () => void;
}) {
  const { t } = useTranslation('navigation');
  return (
    <>
      <div className={styles.previewMeta}>
        <span>{t('page_pane_page_preview')}</span>
        <span>{t('page_pane_object_count', { count })}</span>
      </div>
      <h2 className={styles.previewTitle}>{name}</h2>
      {description && (
        <p
          style={{
            whiteSpace: 'pre-wrap',
            overflowWrap: 'anywhere',
            color: 'var(--text-secondary)',
            fontSize: 12,
          }}
        >
          {description}
        </p>
      )}
      <button type="button" className={styles.jumpButton} onClick={onNavigate}>
        <ArrowRight size={14} />
        {t('page_pane_open_page')}
      </button>
    </>
  );
}

/** Fetch one hovered object locally; never add decrypted details to the persistent navigation index. */
export function PagePaneObjectPreview({
  objectId,
  collectionLabel,
  index,
  total,
  onNavigate,
}: {
  objectId: string;
  collectionLabel: string;
  index: number;
  total: number;
  onNavigate: () => void;
}) {
  const { t } = useTranslation('navigation');
  const accountId = useAuthStore((state) => state.currentAccount?.id);
  const authenticated = useAuthStore((state) => state.isAuthenticated);
  const templates = useTemplateStore((state) => state.templates);
  const requests = useMemo(createSessionRequests, []);
  const [result, setResult] = useState<{
    accountId: string;
    objectId: string;
    object: ObjectDataView | null;
  } | null>(null);
  useEffect(() => {
    if (!accountId || !authenticated) return;
    const request = requests.begin('preview', accountId);
    const unsubscribe = onRequestSessionChange(() => {
      requests.invalidate();
      setResult(null);
    });
    void request
      .invokeTyped('object_get', { accountId, objectId })
      .then((wire) => {
        if (!request.isCurrent()) return;
        const object =
          wire && wire.id === objectId && wire.accountId === accountId && !wire.deletedAt
            ? toObjectDataView(wire)
            : null;
        setResult({ accountId, objectId, object });
      })
      .catch(() => {
        if (request.isCurrent()) setResult({ accountId, objectId, object: null });
      });
    return () => {
      requests.invalidate();
      unsubscribe();
    };
  }, [accountId, authenticated, objectId, requests]);
  if (!authenticated || !accountId) return null;
  const current = result?.accountId === accountId && result.objectId === objectId ? result : null;
  if (!current) return <p role="status">{t('page_pane_loading')}</p>;
  if (!current.object) return <p role="status">{t('page_pane_preview_unavailable')}</p>;
  return (
    <ObjectRulerPreview
      object={current.object}
      template={templates.find(
        (template) =>
          template.id === current.object?.templateId && template.accountId === accountId,
      )}
      collectionLabel={collectionLabel}
      index={index}
      total={total}
      onNavigate={onNavigate}
      navigateLabel={t('page_pane_edit_object')}
    />
  );
}
