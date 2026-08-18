import { useCallback, useMemo } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Button } from '@/components/ui/Button';
import buttonStyles from '@/components/ui/Button.module.css';
import { Search, Trash } from 'lucide-react';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { TemplateSyncConfirmDialog } from '@/components/object/TemplateSyncConfirmDialog';
import { DeprecatedFieldsViewer } from '@/components/object/DeprecatedFieldsViewer';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { useObjectWorkspaceData } from '@/hooks/useObjectWorkspaceData';
import { WorkspaceObjectCard } from './WorkspaceObjectCard';
import type { ObjectSummary, ObjectData } from '@/stores/objectStore';
import { WorkspaceCategoryTabs } from '@/components/workspace/WorkspaceCategoryTabs';
import { ConfirmDeleteDialog } from '@/components/workspace/ConfirmDeleteDialog';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { useWorkspaceGuidePages } from './workspaceGuidePages';
import { ICON_SIZE } from '@/lib/constants';
import styles from './ObjectWorkspacePage.module.css';

/**
 * 对象工作区页：
 * - 数据层（对象加载/搜索/分页/计数/模板同步/密码守卫/字段助手）收敛于 useObjectWorkspaceData hook
 * - 指南静态内容收敛于 useWorkspaceGuidePages
 * - 本组件仅负责视图编排与弹窗挂载
 */
export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pageId } = useParams();
  const sectionFilter = searchParams.get('section') || '';
  const detailObjectId = searchParams.get('objectId');

  const ws = useObjectWorkspaceData({ pageId, sectionFilter, detailObjectId });
  const guidePages = useWorkspaceGuidePages();
  const { t } = ws;
  // 本地别名：属性访问无法在 JSX 守卫中做类型收窄，解构后可对 nullable 值正常 narrow。
  const { detailObj, historyObj, OBJECT_PAGE_SIZE } = ws;
  // V004: fieldOrder 从内联 .find()?.properties.map() 提取为 useMemo——每次渲染新数组引用
  // 会让 HistoryViewer 内部的 useMemo（依赖 [rawProps, fieldOrder]）在异步快照稳定后仍每次失效。
  const historyFieldOrder = useMemo(
    () =>
      ws.userTemplates
        .find((tpl) => tpl.id === historyObj?.templateId)
        ?.properties.map((p) => p.id),
    [ws.userTemplates, historyObj?.templateId],
  );
  // P118: 解构出稳定引用供 useCallback 使用——直接引用 ws 会让 linter 把整个
  // ws 对象列为依赖（ws 每次渲染都是新对象，会把回调打成不稳定）。
  const {
    setDetailObj,
    setHistoryObj,
    setAttachmentObjId,
    setConfirmDelete,
    templateHashMap,
    handleStartSync,
    handleRequestDismissSync,
  } = ws;

  // P118: WorkspaceObjectCard 回调统一接收 obj 参数，父级以 useCallback
  // 提供稳定引用——搜索框每次击键不再新建全部卡片的闭包，memo 不再被击穿
  // （visibleObjects 在防抖期间对象引用稳定，卡片可整体跳过重渲染）。
  const handleCardClick = useCallback(
    (obj: ObjectSummary | ObjectData) => setDetailObj(obj),
    [setDetailObj],
  );
  const handleCardHistory = useCallback(
    (obj: ObjectSummary | ObjectData) =>
      setHistoryObj({
        id: obj.id,
        name: obj.name,
        typeId: obj.typeId,
        templateId: obj.templateId || undefined,
      }),
    [setHistoryObj],
  );
  const handleCardAttachments = useCallback(
    (obj: ObjectSummary | ObjectData) => setAttachmentObjId(obj.id),
    [setAttachmentObjId],
  );
  const handleCardEdit = useCallback(
    (obj: ObjectSummary | ObjectData) => navigate(`/editor/${obj.id}`),
    [navigate],
  );
  const handleCardDelete = useCallback(
    (obj: ObjectSummary | ObjectData) => setConfirmDelete({ id: obj.id, name: obj.name }),
    [setConfirmDelete],
  );
  const handleCardSync = useCallback(
    (obj: ObjectSummary | ObjectData) => handleStartSync(obj.id, obj.name),
    [handleStartSync],
  );
  const handleCardDismissSync = useCallback(
    (obj: ObjectSummary | ObjectData) =>
      handleRequestDismissSync(
        obj.id,
        obj.name,
        obj.templateId ? templateHashMap.get(obj.templateId) : undefined,
      ),
    [handleRequestDismissSync, templateHashMap],
  );

  return (
    <PageShell
      title={ws.customPage?.name || ws.activeCategoryLabel || t('objects')}
      onBack={() => navigate('/home')}
      actions={
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <PageGuideButton pages={guidePages} />
          <button
            className={`${buttonStyles.hideLabelOnMobile} ${styles.createBtn}`}
            onClick={() => navigate(ws.newObjectUrl)}
          >
            + <span className={buttonStyles.label}>{t('create')}</span>
          </button>
          {pageId && ws.customPage && (
            <Button
              variant="danger-outline"
              size="sm"
              className={`${buttonStyles.hideLabelOnMobile} ${buttonStyles.compactMobile}`}
              onClick={() => ws.setConfirmPageDelete(true)}
              title={t('delete')}
            >
              <Trash size={ICON_SIZE.sm} />{' '}
              <span className={buttonStyles.label}>{t('delete')}</span>
            </Button>
          )}
        </div>
      }
    >
      <PageContainer variant="medium" gap="default">
        <div
          className={styles.controls}
          onMouseDown={(e) => {
            if (e.detail > 1) e.preventDefault();
          }}
        >
          <WorkspaceCategoryTabs
            sectionFilter={sectionFilter}
            pageId={pageId}
            customPages={ws.customPages}
            activeCustomPages={ws.activeCustomPages}
            className={styles.tabs}
          />

          <Input
            placeholder={t('search_objects_placeholder')}
            value={ws.searchQuery}
            onChange={(e) => ws.setSearchQuery(e.target.value)}
            onClear={() => ws.setSearchQuery('')}
            prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
          />

          {ws.isLoading && (
            <Card>
              <LoadingPlaceholder variant="elevated" minHeight={80} />
            </Card>
          )}
          {!ws.isLoading && ws.error && (
            <Card>
              <p style={{ textAlign: 'center', color: '#e74c3c', padding: '24px 0' }}>
                {ws.error}
              </p>
            </Card>
          )}
          {!ws.isLoading && !ws.error && ws.visibleObjects.length === 0 && (
            <Card>
              <p
                style={{
                  textAlign: 'center',
                  color: 'var(--text-secondary)',
                  padding: '24px 0',
                  fontSize: 'var(--text-sm)',
                }}
              >
                {ws.searchQuery ? t('no_matching_objects') : t('no_objects')}
              </p>
            </Card>
          )}
          {!ws.isLoading && ws.visibleObjects.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
              {ws.visibleObjects.slice(0, ws.visibleLimit).map((obj) => (
                <WorkspaceObjectCard
                  key={obj.id}
                  obj={obj}
                  collectionLabel={ws.resolveCollectionLabel(obj.typeId)}
                  userTemplates={ws.userTemplates}
                  snapshotCount={ws.snapshotCounts[obj.id]}
                  attachmentCount={ws.attachmentCounts[obj.id]}
                  templateHashMap={ws.templateHashMap}
                  isSyncDialogOpen={ws.syncDialogOpenForObjectId === obj.id}
                  onClick={handleCardClick}
                  onHistory={handleCardHistory}
                  onUploadComplete={ws.refreshAttachmentCounts}
                  onAttachments={handleCardAttachments}
                  onEdit={handleCardEdit}
                  onDelete={handleCardDelete}
                  onSync={handleCardSync}
                  onDismissSync={handleCardDismissSync}
                />
              ))}
              {ws.visibleObjects.length > ws.visibleLimit && (
                <Button
                  variant="tertiary"
                  size="sm"
                  onClick={() => ws.setVisibleLimit((n) => n + OBJECT_PAGE_SIZE)}
                  style={{ marginTop: 4 }}
                >
                  {t('load_more', { defaultValue: '加载更多' })}
                </Button>
              )}
            </div>
          )}

          {/* Page delete confirmation dialog */}
          <ConfirmDeleteDialog
            isOpen={ws.confirmPageDelete && !!pageId && !!ws.customPage}
            title={t('object_delete_confirm_title')}
            body={t('object_delete_confirm_body', {
              name:
                (ws.customPage?.name || '').length > 28
                  ? (ws.customPage?.name || '').slice(0, 27) + '…'
                  : ws.customPage?.name || '',
            })}
            confirmLabel={t('delete')}
            cancelLabel={t('cancel')}
            onCancel={() => ws.setConfirmPageDelete(false)}
            onConfirm={async () => {
              ws.setConfirmPageDelete(false);
              if (ws.accountId && pageId) {
                await ws.removeCustomPage(ws.accountId, pageId);
                navigate('/');
              }
            }}
          />

          {/* Object detail modal */}
          {detailObj && (
            <ObjectDetailModal
              object={detailObj}
              needsSync={ws.detailHashNeedsSync && ws.detailSemanticNeedsSync}
              onClose={() => ws.setDetailObj(null)}
              onEdit={() => {
                navigate(`/editor/${detailObj.id}`);
                ws.setDetailObj(null);
              }}
              onDelete={() => {
                ws.setConfirmDelete({ id: detailObj.id, name: detailObj.name });
                ws.setDetailObj(null);
              }}
              onSyncTemplate={() => ws.handleStartSync(detailObj.id, detailObj.name)}
              onDismissSync={() =>
                ws.handleRequestDismissSync(
                  detailObj.id,
                  detailObj.name,
                  detailObj.templateId ? ws.templateHashMap.get(detailObj.templateId) : undefined,
                )
              }
              onViewDeprecatedFields={() =>
                ws.handleViewDeprecatedFields(detailObj.id, detailObj.name)
              }
              onAttachmentsChange={ws.refreshAttachmentCounts}
            />
          )}

          <ConfirmDeleteDialog
            isOpen={!!ws.confirmDelete}
            title={t('object_delete_confirm_title')}
            body={t('object_delete_confirm_body', {
              name:
                (ws.confirmDelete?.name || '').length > 28
                  ? (ws.confirmDelete?.name || '').slice(0, 27) + '…'
                  : ws.confirmDelete?.name || '',
            })}
            confirmLabel={t('delete')}
            cancelLabel={t('cancel')}
            onCancel={() => ws.setConfirmDelete(null)}
            onConfirm={() => {
              if (ws.confirmDelete) ws.handleDelete(ws.confirmDelete.id);
            }}
          />
        </div>
      </PageContainer>
      {historyObj &&
        (() => {
          const historyObjData = ws.objects.find((o) => o.id === historyObj.id);
          const historyLabels = historyObjData?.propertyLabels;
          const historyFields = (historyObjData?.properties as Record<string, unknown>)
            ?.__fields as Record<string, { name: string }> | undefined;
          return (
            <HistoryViewer
              objectId={historyObj.id}
              objectName={historyObj.name}
              typeId={historyObj.typeId}
              onClose={() => ws.setHistoryObj(null)}
              passwordVerify={ws.passwordVerify}
              getFieldSensitivity={(fieldKey) =>
                ws.getFieldSensitivity(historyObj.templateId, fieldKey, historyLabels)
              }
              isFieldDeprecated={(fieldKey) => ws.isFieldDeprecated(historyObj.templateId, fieldKey)}
              getFieldName={(fieldKey) =>
                ws.getFieldName(historyObj.templateId, fieldKey, historyFields)
              }
              fieldOrder={historyFieldOrder}
            />
          );
        })()}
      {ws.attachmentObjId && (
        <AttachmentViewer
          objectId={ws.attachmentObjId}
          onClose={() => ws.setAttachmentObjId(null)}
          onCountChange={ws.refreshAttachmentCounts}
        />
      )}

      {/* 模板同步确认弹窗 */}
      {ws.syncDialog && (
        <TemplateSyncConfirmDialog
          isOpen={true}
          result={ws.syncDialog.result}
          loading={ws.syncDialog.loading}
          onConfirm={ws.handleConfirmSync}
          onCancel={() => {
            ws.setSyncDialog(null);
            ws.setSyncDialogOpenForObjectId(null);
          }}
        />
      )}

      {/* 忽略模板更新二次确认弹窗 */}
      {ws.dismissConfirm && (
        <ConfirmDeleteDialog
          isOpen={true}
          title={t('editor:template_sync_dismiss_title')}
          body={t('editor:template_sync_dismiss_body')}
          confirmLabel={t('common:confirm')}
          cancelLabel={t('common:cancel')}
          onCancel={() => {
            ws.setDismissConfirm(null);
            ws.setSyncDialogOpenForObjectId(null);
          }}
          onConfirm={ws.handleConfirmDismissSync}
        />
      )}

      {/* 历史字段查看器 */}
      {ws.deprecatedViewer && (
        <DeprecatedFieldsViewer
          isOpen={true}
          objectName={ws.deprecatedViewer.objectName}
          fields={ws.deprecatedFields}
          onClose={() => {
            ws.setDeprecatedViewer(null);
            ws.setDeprecatedFields([]);
          }}
        />
      )}

      {/* Unified password verification dialog (detail panel + history cards) */}
      <PasswordVerificationDialog
        open={ws.showPwDialog}
        onClose={() => {
          ws.setShowPwDialog(false);
          ws.pwResolveRef.current?.({ ok: false, method: 'password' });
        }}
        onVerify={async (password) => {
          const ok = await ws.verifyVaultPassword(password);
          if (ok) ws.pwResolveRef.current?.({ ok: true, method: 'password' });
          return ok;
        }}
        title={t('common:critical_access_title')}
        description={t('common:critical_access_desc')}
        confirmLabel={t('common:unlock')}
        hint={ws.passwordHint}
        pinAccountId={ws.accountId}
        onPinSuccess={() => {
          ws.pwResolveRef.current?.({ ok: true, method: 'password' });
          ws.setShowPwDialog(false);
        }}
        biometricType={ws.bioAvailable.available ? ws.bioAvailable.biometryType : undefined}
        onBiometric={ws.bioAvailable.available ? ws.handleBiometricUnlock : undefined}
      />
    </PageShell>
  );
}
