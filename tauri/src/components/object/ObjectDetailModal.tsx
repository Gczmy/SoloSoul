import { motion } from 'framer-motion';

import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { ObjectDetailFieldsList } from '@/components/object/ObjectDetailFieldsList';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import {
  ObjectDetailHeader,
  ObjectDetailTemplateSyncBanner,
  ObjectDetailDeprecatedEntry,
  ObjectDetailTags,
  ObjectDetailFooter,
} from '@/components/object/ObjectDetailSections';
import { ObjectDetailDeleteDialog } from '@/components/object/ObjectDetailDeleteDialog';
import styles from './ObjectDetailModal.module.css';

import { useObjectDetailModal, type ObjectDetailModalProps } from './useObjectDetailModal';

export type { ObjectDetailModalProps } from './useObjectDetailModal';

/**
 * 对象详情弹窗 — P046 拆分后为纯展示组合层：
 * 全部编排逻辑（对象拉取、字段/敏感度解析、关键数据验证、删除/历史/附件流程）
 * 收敛于 useObjectDetailModal 数据 hook；本组件仅负责 JSX 组合与子组件装配。
 */
export function ObjectDetailModal(props: ObjectDetailModalProps) {
  const {
    needsSync,
    onClose,
    onHistory,
    onAttachments,
    onEdit,
    onDelete,
    onSyncTemplate,
    onDismissSync,
    onViewDeprecatedFields,
    onAttachmentsChange,
  } = props;
  const {
    t,
    accountId,
    // 数据
    loading,
    obj,
    objFieldDefs,
    fields,
    fieldOrder,
    deprecatedFields,
    detailTpl,
    detailTplMatch,
    ObjectDetailIcon,
    resolveCollectionLabelLocal,
    detailGuidePages,
    // 敏感度/字段解析
    getFieldProperty,
    getFieldSensitivity,
    isFieldDeprecated,
    getFieldName,
    // 揭示/复制
    isRevealed,
    maskValue,
    handleRevealField,
    handleCopy,
    copiedField,
    // 删除流程
    confirmDelete,
    setConfirmDelete,
    deleting,
    handleDelete,
    // 历史/附件开关
    showHistory,
    setShowHistory,
    showAttachments,
    setShowAttachments,
    // 关键数据验证
    passwordVerify,
    showPwDialog,
    handlePwDialogClose,
    handlePwDialogVerify,
    handlePwDialogPinSuccess,
    passwordHint,
    bioAvailable,
    handleBiometricUnlock,
    // 拖拽上传
    detailDragRef,
    detailDragState,
  } = useObjectDetailModal(props);

  return (
    <>
      <div className={styles.overlay} onClick={onClose}>
        {!loading && obj && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            ref={detailDragRef}
            onClick={(e) => e.stopPropagation()}
            className={styles.modal}
            data-testid="object-detail-modal"
          >
            <>
              {/* Header */}
              <ObjectDetailHeader
                obj={obj}
                icon={ObjectDetailIcon}
                detailTplMatch={detailTplMatch}
                detailTplName={detailTpl?.name}
                collectionLabel={resolveCollectionLabelLocal(obj.typeId)}
                t={t}
                onClose={onClose}
              />

              <div
                className={styles.headerDivider}
                style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 16 }}
              />

              {/* 可滚动内容区（移动端仅此区域滚动，头尾固定） */}
              <div className={styles.modalBody}>
                {/* 模板更新提示条 */}
                {needsSync && onSyncTemplate && (
                  <ObjectDetailTemplateSyncBanner
                    t={t}
                    onSync={onSyncTemplate}
                    onDismiss={onDismissSync}
                  />
                )}

                {/* 历史字段入口 */}
                {deprecatedFields.length > 0 && onViewDeprecatedFields && (
                  <ObjectDetailDeprecatedEntry
                    t={t}
                    count={deprecatedFields.length}
                    onView={onViewDeprecatedFields}
                  />
                )}

                {/* Fields */}
                {fields.length === 0 ? (
                  <p
                    style={{
                      fontSize: 'var(--text-body-sm)',
                      color: 'var(--text-tertiary)',
                      textAlign: 'center',
                      padding: '16px 0',
                    }}
                  >
                    {t('editor:no_properties')}
                  </p>
                ) : (
                  <ObjectDetailFieldsList
                    fields={fields}
                    typeId={obj.typeId}
                    contractTypeId={obj.contractTypeId}
                    objFieldDefs={objFieldDefs}
                    getFieldProperty={getFieldProperty}
                    getFieldSensitivity={getFieldSensitivity}
                    isFieldDeprecated={isFieldDeprecated}
                    getFieldName={getFieldName}
                    isRevealed={isRevealed}
                    maskValue={maskValue}
                    handleRevealField={handleRevealField}
                    handleCopy={handleCopy}
                    copiedField={copiedField}
                  />
                )}

                {/* 拖拽上传覆盖层 */}
                <DragUploadOverlay dragState={detailDragState} borderRadius={16} />

                {/* Tags */}
                {obj.tags && obj.tags.length > 0 && <ObjectDetailTags tags={obj.tags} />}
              </div>

              {/* Actions */}
              <ObjectDetailFooter
                t={t}
                guidePages={detailGuidePages}
                onHistory={onHistory ?? (() => setShowHistory(true))}
                onAttachments={onAttachments ?? (() => setShowAttachments(true))}
                onEdit={onEdit}
                onDelete={onDelete ?? (() => setConfirmDelete(true))}
              />
            </>
          </motion.div>
        )}
      </div>

      {/* P041: 删除确认对话框提取为子组件 */}
      <ObjectDetailDeleteDialog
        open={confirmDelete && !!obj}
        objectName={obj?.name ?? ''}
        deleting={deleting}
        t={t}
        onCancel={() => setConfirmDelete(false)}
        onConfirm={handleDelete}
      />

      {showHistory && obj && (
        <HistoryViewer
          objectId={obj.id}
          objectName={obj.name}
          typeId={obj.typeId}
          onClose={() => setShowHistory(false)}
          passwordVerify={passwordVerify}
          getFieldSensitivity={getFieldSensitivity}
          isFieldDeprecated={isFieldDeprecated}
          getFieldName={getFieldName}
          fieldOrder={fieldOrder}
          zIndex={5100}
        />
      )}
      {showAttachments && obj && (
        <AttachmentViewer
          objectId={obj.id}
          onClose={() => setShowAttachments(false)}
          onCountChange={onAttachmentsChange}
          zIndex={5100}
        />
      )}

      <PasswordVerificationDialog
        open={showPwDialog}
        onClose={handlePwDialogClose}
        onVerify={handlePwDialogVerify}
        title={t('common:critical_access_title')}
        description={t('common:critical_access_desc')}
        confirmLabel={t('common:unlock')}
        hint={passwordHint}
        pinAccountId={accountId}
        onPinSuccess={handlePwDialogPinSuccess}
        biometricType={bioAvailable.available ? bioAvailable.biometryType : undefined}
        onBiometric={bioAvailable.available ? handleBiometricUnlock : undefined}
      />
    </>
  );
}
