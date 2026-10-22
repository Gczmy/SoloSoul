import { useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { Clock, Paperclip, X, FileText, ChevronRight, LoaderCircle } from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore, type ObjectInspectorTab } from '@/stores/uiStore';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { formatBytes } from '@/lib/utils';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import type { SnapshotEntry } from '@/types/history';
import type { ObjectInspectorContext } from '@/components/layout/shellConfigStore';
import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { useWorkspacePasswordGuard } from '@/hooks/useWorkspacePasswordGuard';
import { useToastError } from '@/hooks/useToastError';
import { Button } from '@/components/ui/Button';
import { HoverScrollLabel } from '@/components/ui/HoverScrollLabel';
import { useIncrementalWindow } from '@/hooks/useIncrementalWindow';
import { HistoryViewer } from './HistoryViewer';
import { useObjectInspectorTransition } from './useObjectInspectorTransition';
import styles from './ObjectInspectorPanel.module.css';

export function ObjectInspectorPanel({
  context,
  onClose,
}: {
  context: ObjectInspectorContext;
  onClose: () => void;
}) {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const [session, setSession] = useState(0);
  useLayoutEffect(() => onRequestSessionChange(() => setSession((n) => n + 1)), []);
  // 交接对象保留框架；账户或锁定会话变化时立即清空整个面板会话。
  return (
    <InspectorSession
      key={JSON.stringify([accountId, session])}
      context={context}
      accountId={accountId}
      onClose={onClose}
    />
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
  const transition = useObjectInspectorTransition(context, tab, accountId);
  const { view, active, pending, failed, objectFailed, showPending, retry } = transition;
  // 标签选中态与已显示的内容一起更新；请求中的旧画面仍保持原来的语义归属。
  const displayedTab = view?.data.tab ?? tab;
  const listRef = useRef<HTMLDivElement>(null);
  const [counts, setCounts] = useState<{
    objectId: string;
    values: Partial<Record<ObjectInspectorTab, number>>;
  }>();
  useLayoutEffect(() => {
    if (!view || !active) return;
    setCounts((previous) => ({
      objectId: view.context.objectId,
      values: {
        ...(previous?.objectId === view.context.objectId ? previous.values : {}),
        [view.data.tab]: view.data.items.length,
      },
    }));
  }, [view, active]);
  const viewKey = active ? view?.key : undefined;
  useLayoutEffect(() => {
    if (viewKey && listRef.current) listRef.current.scrollTop = 0;
  }, [viewKey]);
  return (
    <aside
      className={styles.panel}
      data-object-inspector
      data-object-id={view?.context.objectId ?? context.objectId}
      data-inspector-target-id={context.objectId}
      aria-label={t('object_inspector')}
    >
      <header className={styles.header}>
        <div className={styles.heading}>
          <span>{t('object_inspector')}</span>
          <HoverScrollLabel
            className={styles.name}
            text={view?.context.objectName ?? '\u00a0'}
            focusable={active}
          />
        </div>
        <span className={styles.pending}>
          {showPending && (
            <span role="status" aria-label={t('loading')}>
              <LoaderCircle size={14} aria-hidden="true" />
            </span>
          )}
        </span>
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
          const count =
            view?.data.tab === value
              ? view.data.items.length
              : view && counts?.objectId === view.context.objectId
                ? counts.values[value]
                : undefined;
          return (
            <button
              key={value}
              type="button"
              role="tab"
              id={`object-inspector-tab-${value}`}
              aria-selected={displayedTab === value}
              aria-controls="object-inspector-list"
              tabIndex={displayedTab === value ? 0 : -1}
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
              {count !== undefined && (
                <span className={styles.count} aria-hidden="true">
                  {count}
                </span>
              )}
            </button>
          );
        })}
      </div>
      <div
        ref={listRef}
        className={styles.list}
        id="object-inspector-list"
        role="tabpanel"
        aria-labelledby={`object-inspector-tab-${displayedTab}`}
        tabIndex={0}
        data-object-inspector-scroll
        aria-busy={pending}
      >
        {failed ? (
          <div className={styles.empty} role="alert">
            <p>
              {t(
                objectFailed
                  ? 'object_load_failed'
                  : tab === 'history'
                    ? 'history_load_failed'
                    : 'inspector_attachments_failed',
              )}
            </p>
            {!objectFailed && (
              <Button variant="secondary" size="sm" onClick={retry}>
                {t('retry')}
              </Button>
            )}
          </div>
        ) : view ? (
          <div key={view.key} inert={!active} data-inspector-content>
            {view.data.tab === 'history' ? (
              <InspectorHistory
                context={view.context}
                accountId={accountId}
                initialSnapshots={view.data.items}
                active={active}
              />
            ) : (
              <InspectorAttachments
                context={view.context}
                accountId={accountId}
                initialItems={view.data.items}
                active={active}
              />
            )}
          </div>
        ) : (
          <div className={styles.loading} aria-hidden="true" />
        )}
      </div>
      <footer className={styles.footer}>{t('inspector_newest_first')}</footer>
    </aside>
  );
}

function InspectorHistory({
  context,
  accountId,
  initialSnapshots,
  active,
}: {
  context: ObjectInspectorContext;
  accountId?: string;
  initialSnapshots: SnapshotEntry[];
  active: boolean;
}) {
  const { t } = useTranslation('common');
  const guard = useWorkspacePasswordGuard(accountId);
  const closeVerification = guard.handlePwDialogClose;
  useLayoutEffect(() => {
    if (!active && guard.showPwDialog) closeVerification();
  }, [active, guard.showPwDialog, closeVerification]);
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
        initialSnapshots={initialSnapshots}
        active={active}
      />
      <PasswordVerificationDialog
        open={active && guard.showPwDialog}
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
  initialItems,
  active,
}: {
  context: ObjectInspectorContext;
  accountId?: string;
  initialItems: AttachmentItem[];
  active: boolean;
}) {
  const { t } = useTranslation('common');
  const { onError } = useToastError();
  const [requests] = useState(createSessionRequests);
  const [items, setItems] = useState(initialItems);
  const [preview, setPreview] = useState<AttachmentItem | null>(null);
  const window = useIncrementalWindow(30);
  useLayoutEffect(() => () => requests.invalidate(), [requests]);
  useLayoutEffect(() => {
    if (active) return;
    requests.invalidate();
    setPreview(null);
  }, [active, requests]);
  const openExternal = async (item: AttachmentItem) => {
    if (!active) return;
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
              onClick={() => active && setPreview(item)}
            >
              <span className={styles.fileIcon}>
                <FileText size={19} />
              </span>
              <span className={styles.fileInfo}>
                <HoverScrollLabel className={styles.fileName} text={item.fileName} />
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
      {active &&
        preview &&
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
