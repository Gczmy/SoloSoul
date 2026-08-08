import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { motion } from 'framer-motion';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import { logger } from '@/lib/logger';
import { useObjectStore, type ObjectData, type ObjectSummary } from '@/stores/objectStore';
import { useRevealState } from '@/hooks/useRevealState';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { ObjectDetailFieldsList } from '@/components/object/ObjectDetailFieldsList';
import { resolveCollectionLabel } from '@/lib/utils';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useSettingsStore } from '@/stores/settingsStore';
import type { TemplateProperty } from '@/types/template';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import { isMobilePlatformSync } from '@/lib/platform';
import { flattenProperties, buildDetailGuidePages } from '@/components/object/objectDetailUtils';
import {
  ObjectDetailHeader,
  ObjectDetailTemplateSyncBanner,
  ObjectDetailDeprecatedEntry,
  ObjectDetailTags,
  ObjectDetailFooter,
} from '@/components/object/ObjectDetailSections';
import { ObjectDetailDeleteDialog } from '@/components/object/ObjectDetailDeleteDialog';
import styles from './ObjectDetailModal.module.css';

interface ObjectDetailModalProps {
  /** 已加载的对象摘要/完整数据。与 objectId 二选一，优先使用此值。 */
  object?: ObjectSummary | ObjectData;
  /** 若未提供 object，则通过 objectId 自动拉取完整对象数据。 */
  objectId?: string;
  /** 模板已更新且对象尚未同步时显示提示条。 */
  needsSync?: boolean;
  onClose: () => void;
  onHistory?: () => void;
  onAttachments?: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  /** 用户确认应用模板更新。 */
  onSyncTemplate?: () => void;
  /** 用户选择暂不应用模板更新。 */
  onDismissSync?: () => void;
  /** 查看已归档历史字段。 */
  onViewDeprecatedFields?: () => void;
  /** 附件增删后的回调，用于外部（workspace）刷新计数 badge */
  onAttachmentsChange?: () => void;
}

export function ObjectDetailModal({
  object,
  objectId,
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
}: ObjectDetailModalProps) {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  // P055: 分字段 selector，避免 store 任何变化触发整页重渲染（函数引用稳定）
  const templates = useTemplateStore((s) => s.templates);
  const loadTemplates = useTemplateStore((s) => s.loadTemplates);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const { maskValue, isRevealed, reveal } = useRevealState();
  const [fetchedObj, setFetchedObj] = useState<ObjectData | null>(null);
  const [loading, setLoading] = useState(!object && !!objectId);
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fetchIdRef = useRef(0);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [showAttachments, setShowAttachments] = useState(false);

  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<
    | ((result: {
        ok: boolean;
        method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';
      }) => void)
    | null
  >(null);
  const pendingRevealRef = useRef<{ fieldId: string; fieldName: string } | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  const obj = useMemo(() => object || fetchedObj, [object, fetchedObj]);
  const { ref: detailDragRef, dragState: detailDragState } = useDragToAttach(obj?.id || null, {
    onComplete: onAttachmentsChange,
  });

  useEffect(() => {
    loadTemplates().catch((err) => logger.warn('[ObjectDetail] Load templates failed:', err));
  }, [loadTemplates]);

  useEffect(() => {
    if (!accountId) return;
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      {
        accountId: accountId,
      },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch((err) => logger.warn('[ObjectDetail] Biometric availability check failed:', err));
    invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
      .then((accounts) => {
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      })
      .catch((err) => logger.warn('[ObjectDetail] Load password hint failed:', err));
  }, [accountId]);

  useEffect(() => {
    if (object || !objectId || !accountId) {
      if (!object && !objectId) setLoading(false);
      return;
    }
    const id = ++fetchIdRef.current;
    setLoading(true);
    useObjectStore
      .getState()
      .getObject(accountId, objectId)
      .then(() => {
        // Discard stale responses: if a newer fetch started while this one
        // was in-flight, don't overwrite fetchedObj with potentially stale data.
        if (fetchIdRef.current !== id) return;
        setFetchedObj(useObjectStore.getState().currentObjectCache[objectId] ?? null);
      })
      .catch(() => {
        if (fetchIdRef.current === id) setFetchedObj(null);
      })
      .finally(() => {
        if (fetchIdRef.current === id) setLoading(false);
      });
  }, [object, objectId, accountId]);

  const unlockVaultWithPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        await invoke('unlock_with_password', { accountId: accountId, password });
        return true;
      } catch (err) {
        // P124: 密码错误与后端异常可区分——后端对错误密码返回 Err("Invalid password")，
        // 返回 false（对话框显示「密码不正确」）；其余为真实后端异常，抛出保留细节
        // （对话框 catch 走 onError toast），不再无差别当作密码错误。
        const msg =
          typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
        if (/invalid password|incorrect password|密码错误|密码不正确/i.test(msg)) {
          return false;
        }
        logger.warn('[ObjectDetail] Vault unlock failed:', err);
        throw err;
      }
    },
    [accountId],
  );

  const passwordVerify = useCallback(async (): Promise<{
    ok: boolean;
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';
  }> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  const resolveCollectionLabelLocal = useCallback(
    (typeId: string) => resolveCollectionLabel(typeId, customPages, t),
    [customPages, t],
  );

  const writeCriticalAccessLog = useCallback(
    async (method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin') => {
      if (!accountId || !obj || !pendingRevealRef.current) return;
      const actionType =
        method === 'password'
          ? 'critical_field_login'
          : method === 'pin'
            ? 'critical_field_pin'
            : method === 'touchId'
              ? 'critical_field_touch_id'
              : method === 'windowsHello'
                ? 'critical_field_windows_hello'
                : 'critical_field_face_id';
      const entityType = method === 'password' || method === 'pin' ? 'auth' : 'biometric';
      const details = `objectName=${obj.name} page=${resolveCollectionLabelLocal(obj.typeId)} fieldName=${pendingRevealRef.current.fieldName}`;
      try {
        await invoke('log_write', {
          request: {
            actionType,
            entityType,
            entityId: obj.id,
            entityName: null,
            details,
          },
        });
      } catch {
        // best effort
      }
    },
    [accountId, obj, resolveCollectionLabelLocal],
  );

  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', {
        accountId: accountId,
        location: 'critical_data_access',
        action: 'unlock',
        biometryType: bioAvailable.biometryType,
      });
      const method =
        (bioAvailable.biometryType as 'touchId' | 'faceId' | 'windowsHello') || 'touchId';
      pwResolveRef.current?.({ ok: true, method });
      return true;
    } catch (err) {
      // P124: 记录失败细节（用户取消 vs 后端异常在 UI 上保持静默停留，但日志不再丢失）
      logger.warn('[ObjectDetail] Biometric unlock failed:', err);
      return false;
    }
  }, [accountId, bioAvailable.biometryType]);

  const handleRevealField = useCallback(
    async (fieldId: string, sens: SensitivityLevel, fieldName: string) => {
      if (sens === 'critical') {
        pendingRevealRef.current = { fieldId, fieldName };
        const result = await passwordVerify();
        if (result.ok) {
          reveal(fieldId);
          await writeCriticalAccessLog(result.method);
        }
      } else {
        reveal(fieldId);
      }
    },
    [passwordVerify, reveal, writeCriticalAccessLog],
  );

  // F012: cache the current object's template field map for O(1) lookups.
  const fieldMap = useMemo(() => {
    const tpl = templates.find((t) => t.id === obj?.templateId);
    return new Map<string, TemplateProperty>(tpl?.properties.map((p) => [p.id, p]) ?? []);
  }, [templates, obj?.templateId]);

  // 从 properties 中提取 __fields（即使模板被删除，字段定义仍保留在对象上）
  const objFieldDefs = useMemo(() => {
    const raw = (obj?.properties as Record<string, unknown>)?.__fields;
    return raw as Record<string, { name: string; type: string }> | undefined;
  }, [obj?.properties]);

  // 已归档的历史字段（模板字段类型不兼容变更时产生）
  const deprecatedFields = useMemo(() => {
    const raw = (obj?.properties as Record<string, unknown>)?.__deprecatedFields;
    return Array.isArray(raw) ? raw : [];
  }, [obj?.properties]);

  const getFieldProperty = (fieldKey: string): TemplateProperty | undefined => {
    return fieldMap.get(fieldKey);
  };

  const getFieldSensitivity = (fieldKey: string): SensitivityLevel => {
    // 1. 对象自有 propertyLabels（即使模板被删除也保留敏感度）
    const labels = obj?.propertyLabels as Record<string, string> | undefined;
    if (labels?.[fieldKey]) {
      return labels[fieldKey] as SensitivityLevel;
    }
    // 2. 回退到模板定义
    return (getFieldProperty(fieldKey)?.sensitivityLevel as SensitivityLevel) || 'internal';
  };

  const isFieldDeprecated = (fieldKey: string): boolean => {
    return !!getFieldProperty(fieldKey)?.deprecatedAt;
  };

  const getFieldName = (fieldKey: string, label?: string): string => {
    if (label) return label;
    return getFieldProperty(fieldKey)?.name || objFieldDefs?.[fieldKey]?.name || fieldKey;
  };

  const handleCopy = async (value: string, key: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopiedField(key);
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
      copyTimeoutRef.current = setTimeout(() => setCopiedField(null), COPY_FEEDBACK_DURATION_MS);
    } catch {
      /* ignore */
    }
  };

  const handleDelete = async () => {
    if (onDelete) {
      onDelete();
      return;
    }
    if (!obj) return;
    setDeleting(true);
    try {
      await useObjectStore.getState().deleteObject(obj.id);
      onClose();
      setConfirmDelete(false);
    } catch (err) {
      // P006: 删除失败不得静默——保持确认弹窗打开并提示，避免用户
      // 以为删除成功而对象仍存在。
      import('@/stores/uiStore').then(({ useUiStore }) => {
        useUiStore.getState().showToast({
          type: 'error',
          message: `${t('common:delete_failed')}: ${err}`, // 具体错误便于诊断
          duration: 5000,
        });
      });
      logger.warn('[ObjectDetail] Delete object failed:', err);
    } finally {
      setDeleting(false);
    }
  };

  const detailTpl = obj?.templateId ? templates.find((t) => t.id === obj.templateId) : undefined;
  // 模板匹配需同时满足 ID 和页面归属（与编辑器 ObjectEditorPage 对齐）。
  // !! 归一化为 boolean：原实现仅用于三元/条件真值判断，语义等价。
  const detailTplMatch = !!detailTpl && (detailTpl.category || 'identity') === obj?.typeId;
  const ObjectDetailIcon = detailTpl?.iconId
    ? resolveCustomIcon(detailTpl.iconId)
    : PAGE_ICON_MAP.custom;
  const fieldOrder = templates.find((t) => t.id === obj?.templateId)?.properties.map((p) => p.id);
  const fields = flattenProperties(obj?.properties, fieldOrder, objFieldDefs);

  const isMobilePlatform = isMobilePlatformSync();

  const detailGuidePages = useMemo(
    () => buildDetailGuidePages(t, isMobilePlatform),
    [t, isMobilePlatform],
  );

  // 操作按钮样式已迁移至 ObjectDetailModal.module.css，避免 inline hover 在触屏残留高亮

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
        onClose={() => {
          setShowPwDialog(false);
          pwResolveRef.current?.({ ok: false, method: 'password' });
        }}
        onVerify={async (password) => {
          const ok = await unlockVaultWithPassword(password);
          if (ok) pwResolveRef.current?.({ ok: true, method: 'password' });
          return ok;
        }}
        title={t('common:critical_access_title')}
        description={t('common:critical_access_desc')}
        confirmLabel={t('common:unlock')}
        hint={passwordHint}
        pinAccountId={accountId}
        onPinSuccess={() => {
          pwResolveRef.current?.({ ok: true, method: 'pin' });
          setShowPwDialog(false);
        }}
        biometricType={bioAvailable.available ? bioAvailable.biometryType : undefined}
        onBiometric={bioAvailable.available ? handleBiometricUnlock : undefined}
      />
    </>
  );
}
