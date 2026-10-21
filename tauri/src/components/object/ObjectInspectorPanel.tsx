import { useEffect, useLayoutEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { Clock, Paperclip, X, FileText, ChevronRight, LoaderCircle } from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore, type ObjectInspectorTab } from '@/stores/uiStore';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { formatBytes } from '@/lib/utils';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import type { ObjectInspectorContext } from '@/components/layout/shellConfigStore';
import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { useWorkspacePasswordGuard } from '@/hooks/useWorkspacePasswordGuard';
import { useToastError } from '@/hooks/useToastError';
import { Button } from '@/components/ui/Button';
import { useIncrementalWindow } from '@/hooks/useIncrementalWindow';
import { HistoryViewer } from './HistoryViewer';
import styles from './ObjectInspectorPanel.module.css';

export function ObjectInspectorPanel({
  context,
  onClose,
}: {
  context: ObjectInspectorContext;
  onClose: () => void;
}) {
  const { t } = useTranslation('common');
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const [session, setSession] = useState(0);
  useLayoutEffect(() => onRequestSessionChange(() => setSession((n) => n + 1)), []);
  // 对象尚未就绪时不沿用上一个对象的名称、正文、附件和验证对话框。
  return context.ready ? (
    <InspectorSession
      key={JSON.stringify([accountId, context.objectId, session])}
      context={context}
      accountId={accountId}
      onClose={onClose}
    />
  ) : (
    <div className={styles.loading} role={context.failed ? 'alert' : 'status'}>
      {!context.failed && <LoaderCircle size={18} aria-hidden="true" />}
      {t(context.failed ? 'object_load_failed' : 'loading')}
    </div>
  );
}

function InspectorSession({
  context,
  accountId,
  onClose,
}: {
  context: ObjectInspectorContext;
  accountId?: string;
  onClose: () => void;
}) {
  const { t } = useTranslation('common');
  const tab = useUiStore((s) => s.desktopObjectInspectorTab);
  const setTab = useUiStore((s) => s.setDesktopObjectInspectorTab);
  const [counts, setCounts] = useState<Partial<Record<ObjectInspectorTab, number>>>({});
  return (
    <aside
      className={styles.panel}
      data-object-inspector
      data-object-id={context.objectId}
      aria-label={t('object_inspector')}
    >
      <header className={styles.header}>
        <div className={styles.heading}>
          <span>{t('object_inspector')}</span>
          <span className={styles.name} title={context.objectName}>
            {context.objectName}
          </span>
        </div>
        <button
          type="button"
          data-ui-icon-button="neutral"
          data-ui-icon-intent="neutral"
          className={styles.close}
          aria-label={t('close_object_inspector')}
          title={t('close')}
          onClick={onClose}
        >
          <X size={16} />
        </button>
      </header>
      <div role="tablist" aria-label={t('object_inspector')} className={styles.tabs}>
        {(['history', 'attachments'] as const).map((value) => {
          const Icon = value === 'history' ? Clock : Paperclip;
          return (
            <button
              key={value}
              type="button"
              role="tab"
              id={`object-inspector-tab-${value}`}
              aria-selected={tab === value}
              aria-controls="object-inspector-list"
              tabIndex={tab === value ? 0 : -1}
              onKeyDown={(event) => {
                if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
                event.preventDefault();
                const next =
                  event.key === 'Home'
                    ? 'history'
                    : event.key === 'End'
                      ? 'attachments'
                      : value === 'history'
                        ? 'attachments'
                        : 'history';
                setTab(next);
                document.getElementById(`object-inspector-tab-${next}`)?.focus();
              }}
              onClick={() => setTab(value)}
            >
              <Icon size={14} aria-hidden="true" />
              {t(value)}
              {counts[value] !== undefined && (
                <span className={styles.count} aria-hidden="true">
                  {counts[value]}
                </span>
              )}
            </button>
          );
        })}
      </div>
      <div
        className={styles.list}
        key={tab}
        id="object-inspector-list"
        role="tabpanel"
        aria-labelledby={`object-inspector-tab-${tab}`}
        tabIndex={0}
        data-object-inspector-scroll
      >
        {tab === 'history' ? (
          <InspectorHistory
            context={context}
            accountId={accountId}
            onCount={(count) =>
              setCounts((previous) =>
                previous.history === count ? previous : { ...previous, history: count },
              )
            }
          />
        ) : (
          <InspectorAttachments
            context={context}
            accountId={accountId}
            onCount={(count) =>
              setCounts((previous) =>
                previous.attachments === count ? previous : { ...previous, attachments: count },
              )
            }
          />
        )}
      </div>
      <footer className={styles.footer}>{t('inspector_newest_first')}</footer>
    </aside>
  );
}

function InspectorHistory({
  context,
  accountId,
  onCount,
}: {
  context: ObjectInspectorContext;
  accountId?: string;
  onCount: (count: number) => void;
}) {
  const { t } = useTranslation('common');
  const guard = useWorkspacePasswordGuard(accountId);
  return (
    <>
      <HistoryViewer
        presentation="timeline"
        objectId={context.objectId}
        objectName={context.objectName}
        typeId={context.typeId}
        onClose={() => {}}
        passwordVerify={guard.passwordVerify}
        getFieldSensitivity={() => 'internal'}
        getFieldName={(key) => key}
        isFieldDeprecated={() => false}
        onCountChange={onCount}
      />
      <PasswordVerificationDialog
        open={guard.showPwDialog}
        onClose={guard.handlePwDialogClose}
        onVerify={guard.handlePwDialogVerify}
        title={t('critical_access_title')}
        description={t('critical_access_desc')}
        confirmLabel={t('unlock')}
        hint={guard.passwordHint}
        pinAccountId={accountId}
        onPinSuccess={guard.handlePwDialogPinSuccess}
        biometricType={guard.bioAvailable.available ? guard.bioAvailable.biometryType : undefined}
        onBiometric={guard.bioAvailable.available ? guard.handleBiometricUnlock : undefined}
      />
    </>
  );
}

function InspectorAttachments({
  context,
  accountId,
  onCount,
}: {
  context: ObjectInspectorContext;
  accountId?: string;
  onCount: (count: number) => void;
}) {
  const { t } = useTranslation('common');
  const { onError } = useToastError();
  const [requests] = useState(createSessionRequests);
  const [items, setItems] = useState<AttachmentItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [preview, setPreview] = useState<AttachmentItem | null>(null);
  const window = useIncrementalWindow(30);
  useLayoutEffect(() => () => requests.invalidate(), [requests]);
  useEffect(() => {
    const request = requests.begin('attachments', accountId);
    setLoading(true);
    setFailed(false);
    request
      .invoke<AttachmentItem[]>('attachment_list', {
        objectId: context.objectId,
        showDeleted: false,
      })
      .then((data) => {
        if (!request.isCurrent()) return;
        const timestamp = (item: AttachmentItem) => Date.parse(item.createdAt) || 0;
        setItems(
          data
            .filter((item) => item.objectId === context.objectId && !item.deletedAt)
            .sort((a, b) => timestamp(b) - timestamp(a)),
        );
      })
      .catch(() => {
        if (request.isCurrent()) setFailed(true);
      })
      .finally(() => {
        if (request.isCurrent()) setLoading(false);
      });
    return () => requests.invalidate('attachments');
  }, [context.objectId, accountId, requests, attempt]);
  useEffect(() => {
    if (!loading && !failed) onCount(items.length);
  }, [loading, failed, items.length, onCount]);
  const openExternal = async (item: AttachmentItem) => {
    const request = requests.begin(undefined, accountId);
    try {
      await request.invoke('attachment_open', {
        objectId: context.objectId,
        attachmentId: item.id,
      });
    } catch (error) {
      if (request.isCurrent()) onError(error, t('inspector_attachments_failed'));
    }
  };
  if (loading)
    return (
      <div className={styles.loading} role="status">
        <LoaderCircle size={18} />
        {t('loading')}
      </div>
    );
  if (failed)
    return (
      <div className={styles.empty} role="alert">
        <p>{t('inspector_attachments_failed')}</p>
        <Button variant="secondary" size="sm" onClick={() => setAttempt((n) => n + 1)}>
          {t('retry')}
        </Button>
      </div>
    );
  return (
    <>
      {items.length === 0 ? (
        <div className={styles.empty}>
          <Paperclip size={28} />
          <p>{t('no_attachments')}</p>
        </div>
      ) : (
        items.slice(0, window.limit).map((item) => {
          const date = new Date(item.createdAt);
          return (
            <button
              type="button"
              className={styles.file}
              key={item.id}
              data-inspector-attachment={item.id}
              onClick={() => setPreview(item)}
            >
              <span className={styles.fileIcon}>
                <FileText size={19} />
              </span>
              <span className={styles.fileInfo}>
                <span className={styles.fileName} title={item.fileName}>
                  {item.fileName}
                </span>
                <span className={styles.fileMeta}>
                  {formatBytes(item.sizeBytes)} ·{' '}
                  <time dateTime={item.createdAt}>
                    {Number.isNaN(date.getTime())
                      ? '—'
                      : date.toLocaleString([], {
                          month: 'short',
                          day: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit',
                        })}
                  </time>
                </span>
              </span>
              <ChevronRight size={14} aria-hidden="true" />
            </button>
          );
        })
      )}
      {window.hasMore(items.length) && (
        <Button size="sm" variant="secondary" onClick={window.showMore}>
          {t('load_more')}
        </Button>
      )}
      {preview &&
        createPortal(
          <AttachmentPreviewOverlay
            item={preview}
            onClose={() => setPreview(null)}
            onOpenExternal={(item) => void openExternal(item)}
            onItemUpdated={(item) => {
              if (item.objectId !== context.objectId) return;
              setItems((previous) => previous.map((old) => (old.id === item.id ? item : old)));
              setPreview(item);
            }}
          />,
          document.body,
        )}
    </>
  );
}
