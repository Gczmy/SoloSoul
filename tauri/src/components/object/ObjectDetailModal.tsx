import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import {
  X,
  Clock,
  Paperclip,
  Pencil,
  Lock,
  Eye,
  Copy,
  Check,
  Maximize2,
  Upload,
} from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useObjectStore, type ObjectData, type ObjectSummary } from '@/stores/objectStore';
import { useRevealState } from '@/hooks/useRevealState';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Button } from '@/components/ui/Button';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { PluginBadge } from '@/components/template/PluginBadge';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { resolveCollectionLabel } from '@/lib/utils';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useSettingsStore } from '@/stores/settingsStore';
import type { TemplateProperty } from '@/types/template';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import { PageGuide } from '@/components/guide/PageGuide';
import { ICON_SIZE } from '@/lib/constants';

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

function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
  fieldDefs?: Record<string, { type?: string }>,
): { key: string; label?: string; value: string; fieldId?: string }[] {
  if (!props) return [];
  const entries: { key: string; label?: string; value: string; fieldId?: string }[] = [];
  const defs = fieldDefs ??
    ((props.__fields as Record<string, { type?: string }> | undefined) || {});
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith('__')) continue;
    if (v === null || v === undefined || v === '') continue;
    const fieldType = defs[k]?.type;
    if (fieldType === 'dynamic_group' && Array.isArray(v)) {
      for (const item of v) {
        if (!item || typeof item !== 'object') continue;
        const { id, name, value } = item as Record<string, unknown>;
        if (name === undefined || name === null || name === '') continue;
        let displayValue = '';
        if (Array.isArray(value)) {
          displayValue = value.join(', ');
        } else if (value !== null && value !== undefined) {
          displayValue = String(value);
        }
        entries.push({
          key: k,
          label: String(name),
          value: displayValue,
          fieldId: id ? `${k}.${id}` : `${k}.${name}`,
        });
      }
    } else if (typeof v === 'string') {
      entries.push({ key: k, value: v });
    } else if (typeof v === 'number' || typeof v === 'boolean') {
      entries.push({ key: k, value: String(v) });
    } else if (Array.isArray(v) && v.length > 0) {
      entries.push({ key: k, value: v.join(', ') });
    }
  }
  if (fieldOrder && fieldOrder.length > 0) {
    const orderMap = new Map(fieldOrder.map((id, i) => [id, i]));
    entries.sort((a, b) => {
      const ia = orderMap.get(a.key);
      const ib = orderMap.get(b.key);
      if (ia !== undefined && ib !== undefined) return ia - ib;
      if (ia !== undefined) return -1;
      if (ib !== undefined) return 1;
      return a.key.localeCompare(b.key);
    });
  }
  return entries;
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
  const { templates, loadTemplates } = useTemplateStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const { maskValue, isRevealed, reveal } = useRevealState();
  const [fetchedObj, setFetchedObj] = useState<ObjectData | null>(null);
  const [loading, setLoading] = useState(!object && !!objectId);
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const [hoveredField, setHoveredField] = useState<string | null>(null);
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
    loadTemplates().catch((err) => console.warn('[ObjectDetail] Load templates failed:', err));
  }, [loadTemplates]);

  useEffect(() => {
    if (!accountId) return;
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      {
        accountId,
      },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch((err) => console.warn('[ObjectDetail] Biometric availability check failed:', err));
    invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
      .then((accounts) => {
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      })
      .catch((err) => console.warn('[ObjectDetail] Load password hint failed:', err));
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
        await invoke('unlock_with_password', { accountId, password });
        return true;
      } catch {
        return false;
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
    (collectionType: string) => resolveCollectionLabel(collectionType, customPages, t),
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
      const details = `objectName=${obj.name} page=${resolveCollectionLabelLocal(obj.collectionType)} fieldName=${pendingRevealRef.current.fieldName}`;
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
        accountId,
        location: 'critical_data_access',
        action: 'unlock',
        biometryType: bioAvailable.biometryType,
      });
      const method =
        (bioAvailable.biometryType as 'touchId' | 'faceId' | 'windowsHello') || 'touchId';
      pwResolveRef.current?.({ ok: true, method });
      return true;
    } catch {
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

  const getFieldType = (fieldKey: string): string => {
    return getFieldProperty(fieldKey)?.type || objFieldDefs?.[fieldKey]?.type || 'text';
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
    } catch {
      /* ignore */
    } finally {
      setDeleting(false);
      setConfirmDelete(false);
    }
  };

  const detailTpl = obj?.templateId ? templates.find((t) => t.id === obj.templateId) : undefined;
  // 模板匹配需同时满足 ID 和页面归属（与编辑器 ObjectEditorPage 对齐）
  const detailTplMatch = detailTpl && (detailTpl.category || 'identity') === obj?.collectionType;
  const ObjectDetailIcon = detailTpl?.iconId
    ? resolveCustomIcon(detailTpl.iconId)
    : PAGE_ICON_MAP.custom;
  const fieldOrder = templates.find((t) => t.id === obj?.templateId)?.properties.map((p) => p.id);
  const fields = flattenProperties(obj?.properties, fieldOrder, objFieldDefs);

  const actionBtnStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    padding: '8px 12px',
    borderRadius: 8,
    border: '1px solid var(--border-subtle)',
    background: 'var(--bg-toolbar)',
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    fontSize: 'var(--text-body-sm)',
    fontWeight: 500,
    transition: 'all 0.15s ease',
  };

  const onActionBtnEnter = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
    e.currentTarget.style.borderColor = 'var(--accent-primary)';
    e.currentTarget.style.color = 'var(--accent-primary)';
  };
  const onActionBtnLeave = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background = 'var(--bg-toolbar)';
    e.currentTarget.style.borderColor = 'var(--border-subtle)';
    e.currentTarget.style.color = 'var(--text-secondary)';
  };
  // Removed onDeleteBtnEnter/onDeleteBtnLeave — now using DeleteButton

  return (
    <>
      <div
        style={{
          position: 'fixed',
          inset: 0,
          zIndex: 3000,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'var(--bg-overlay)',
          backdropFilter: 'blur(4px)',
        }}
        onClick={onClose}
      >
        {!loading && obj && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            ref={detailDragRef}
            onClick={(e) => e.stopPropagation()}
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 16,
              padding: '28px 32px',
              maxWidth: 560,
              width: '90%',
              maxHeight: '80vh',
              overflowY: 'auto',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
              position: 'relative',
            }}
          >
            <>
              {/* Header */}
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  marginBottom: 20,
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <span style={{ flexShrink: 0, display: 'flex' }}>
                    <ObjectDetailIcon size={ICON_SIZE['2xl']} />
                  </span>
                  <div>
                    <h2
                      style={{
                        fontSize: 'var(--text-md)',
                        fontWeight: 700,
                        margin: 0,
                        overflowWrap: 'break-word',
                        wordBreak: 'break-word',
                      }}
                    >
                      {obj.name}
                    </h2>
                    <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {resolveCollectionLabelLocal(obj.collectionType)}
                      {obj.contractTypeId && (
                        <span
                          style={{ marginLeft: 4, display: 'inline-flex', verticalAlign: 'middle' }}
                        >
                          <PluginBadge
                            contractTypeId={obj.contractTypeId}
                            size="sm"
                            variant="full"
                          />
                        </span>
                      )}
                      {/* 模板名 — 模板不匹配（已删除/更改页面）时显示删除线 */}
                      {obj.templateId &&
                        (() => {
                          const tplName = (obj.properties as Record<string, unknown>)
                            ?.__templateName as string | undefined;
                          const tid = obj.templateId || '';
                          const label = detailTplMatch
                            ? detailTpl?.name || tid
                            : tplName
                              ? `${tplName} (${tid.slice(0, 8)}…)`
                              : tid;
                          return (
                            <span
                              style={{ textDecoration: detailTplMatch ? 'none' : 'line-through' }}
                            >
                              {' · '}
                              {label}
                            </span>
                          );
                        })()}
                      {' · '}
                      {t('common:created')}: {obj.createdAt?.slice(0, 10) || '—'} ·{' '}
                      {t('common:updated')}: {obj.updatedAt?.slice(0, 10) || '—'}
                    </span>
                  </div>
                </div>
                <button
                  onClick={onClose}
                  style={{
                    padding: 6,
                    borderRadius: 8,
                    border: 'none',
                    background: 'transparent',
                    cursor: 'pointer',
                    color: 'var(--text-tertiary)',
                    transition: 'all 0.15s ease',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'transparent';
                    e.currentTarget.style.color = 'var(--text-tertiary)';
                  }}
                >
                  <X size={ICON_SIZE.xl} />
                </button>
              </div>

              <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 16 }} />

              {/* 模板更新提示条 */}
              {needsSync && onSyncTemplate && (
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: 12,
                    marginBottom: 16,
                    padding: '10px 12px',
                    borderRadius: 8,
                    background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                    border: '1px solid color-mix(in srgb, var(--accent-primary) 25%, transparent)',
                  }}
                >
                  <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-primary)' }}>
                    {t('editor:template_updated_hint')}
                  </span>
                  <div style={{ display: 'flex', gap: 8, flexShrink: 0 }}>
                    <button
                      onClick={onSyncTemplate}
                      style={{
                        padding: '4px 10px',
                        borderRadius: 6,
                        border: 'none',
                        background: 'var(--accent-primary)',
                        color: '#fff',
                        fontSize: 'var(--text-caption)',
                        fontWeight: 600,
                        cursor: 'pointer',
                      }}
                    >
                      {t('common:yes')}
                    </button>
                    <button
                      onClick={onDismissSync}
                      style={{
                        padding: '4px 10px',
                        borderRadius: 6,
                        border: '1px solid var(--border-subtle)',
                        background: 'var(--bg-elevated)',
                        color: 'var(--text-secondary)',
                        fontSize: 'var(--text-caption)',
                        fontWeight: 500,
                        cursor: 'pointer',
                      }}
                    >
                      {t('common:no')}
                    </button>
                  </div>
                </div>
              )}

              {/* 历史字段入口 */}
              {deprecatedFields.length > 0 && onViewDeprecatedFields && (
                <div style={{ marginBottom: 12 }}>
                  <button
                    onClick={onViewDeprecatedFields}
                    style={{
                      padding: '6px 10px',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: 'var(--bg-toolbar)',
                      color: 'var(--text-secondary)',
                      fontSize: 'var(--text-caption)',
                      cursor: 'pointer',
                    }}
                  >
                    {t('editor:deprecated_fields_button', { count: deprecatedFields.length })}
                  </button>
                </div>
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
                <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                  {fields.map((f) => {
                    const sens = getFieldSensitivity(f.key);
                    const deprecated = isFieldDeprecated(f.key);
                    const fieldId = f.fieldId || `${obj.collectionType}.${f.key}`;
                    const revealed = isRevealed(fieldId);
                    const needsReveal = sens === 'sensitive' || sens === 'critical';
                    return (
                      <div
                        key={f.key}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          gap: 12,
                          padding: '8px 12px',
                          borderRadius: 8,
                          background: 'var(--bg-toolbar)',
                          border: '1px solid var(--border-subtle)',
                          opacity: deprecated ? 0.7 : 1,
                        }}
                      >
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 6,
                              marginBottom: 2,
                            }}
                          >
                            <span
                              style={{
                                fontSize: 'var(--text-caption)',
                                fontWeight: 600,
                                color: 'var(--text-secondary)',
                                textDecoration: deprecated ? 'line-through' : 'none',
                              }}
                            >
                              {getFieldName(f.key, f.label)}
                            </span>
                            <SensitivityBadge level={sens} />
                            {obj.contractTypeId && (
                              <PluginBadge
                                contractTypeId={obj.contractTypeId}
                                size="sm"
                                variant="full"
                              />
                            )}
                            {deprecated && <DeprecatedBadge />}
                          </div>
                          <div
                            style={{
                              fontSize: 'var(--text-body)',
                              color:
                                needsReveal && !revealed
                                  ? 'var(--text-tertiary)'
                                  : 'var(--text-primary)',
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                              whiteSpace: 'nowrap',
                            }}
                          >
                            {revealed ? f.value : maskValue(f.value, fieldId, sens)}
                          </div>
                        </div>
                        <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
                          {needsReveal && !revealed && (
                            <button
                              onClick={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                handleRevealField(fieldId, sens, getFieldName(f.key));
                              }}
                              style={{
                                padding: '4px 10px',
                                borderRadius: 6,
                                border: '1px solid var(--border-subtle)',
                                background:
                                  sens === 'critical' ? 'rgba(220,38,38,0.06)' : 'transparent',
                                cursor: 'pointer',
                                fontSize: 'var(--text-badge)',
                                color: sens === 'critical' ? '#dc2626' : 'var(--text-tertiary)',
                                display: 'flex',
                                alignItems: 'center',
                                gap: 4,
                                transition: 'all 0.15s ease',
                              }}
                              onMouseEnter={(e) => {
                                if (sens === 'critical') {
                                  e.currentTarget.style.background = 'rgba(220,38,38,0.12)';
                                } else {
                                  e.currentTarget.style.background =
                                    'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                                  e.currentTarget.style.color = 'var(--accent-primary)';
                                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                                }
                              }}
                              onMouseLeave={(e) => {
                                if (sens === 'critical') {
                                  e.currentTarget.style.background = 'rgba(220,38,38,0.06)';
                                } else {
                                  e.currentTarget.style.background = 'transparent';
                                  e.currentTarget.style.color = 'var(--text-tertiary)';
                                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                                }
                              }}
                            >
                              {sens === 'critical' ? (
                                <Lock size={ICON_SIZE.xs} />
                              ) : (
                                <Eye size={ICON_SIZE.xs} />
                              )}{' '}
                              {sens === 'critical' ? t('common:unlock') : t('common:reveal')}
                            </button>
                          )}
                          <button
                            onMouseDown={(e) => e.preventDefault()}
                            onClick={() =>
                              handleCopy(
                                revealed ? f.value : maskValue(f.value, fieldId, sens),
                                f.key,
                              )
                            }
                            onMouseEnter={() => setHoveredField(f.key)}
                            onMouseLeave={() => setHoveredField(null)}
                            style={{
                              padding: '4px 10px',
                              borderRadius: 6,
                              border:
                                '1px solid ' +
                                (copiedField === f.key
                                  ? 'var(--accent-primary)'
                                  : 'var(--border-subtle)'),
                              background:
                                hoveredField === f.key && copiedField !== f.key
                                  ? 'color-mix(in srgb, var(--accent-primary) 12%, transparent)'
                                  : 'transparent',
                              cursor: 'pointer',
                              fontSize: 'var(--text-badge)',
                              color:
                                copiedField === f.key
                                  ? 'var(--accent-primary)'
                                  : hoveredField === f.key
                                    ? 'var(--accent-primary)'
                                    : 'var(--text-tertiary)',
                              display: 'flex',
                              alignItems: 'center',
                              gap: 4,
                              boxShadow:
                                copiedField === f.key
                                  ? '0 0 10px color-mix(in srgb, var(--accent-primary) 35%, transparent)'
                                  : 'none',
                              transition: 'all var(--duration-fast) var(--ease-smooth)',
                            }}
                          >
                            {copiedField === f.key ? (
                              <Check size={ICON_SIZE.xs} />
                            ) : (
                              <Copy size={ICON_SIZE.xs} />
                            )}
                            {copiedField === f.key ? t('common:copied') : t('common:copy')}
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}

              {/* 拖拽上传覆盖层 */}
              <DragUploadOverlay dragState={detailDragState} borderRadius={16} />

              {/* Tags */}
              {obj.tags && obj.tags.length > 0 && (
                <div style={{ marginTop: 16, display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                  {obj.tags.map((tag: string) => (
                    <span
                      key={tag}
                      style={{
                        padding: '2px 8px',
                        borderRadius: 10,
                        fontSize: 'var(--text-badge)',
                        background: 'var(--bg-toolbar)',
                        color: 'var(--text-secondary)',
                        border: '1px solid var(--border-subtle)',
                      }}
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              )}

              {/* Actions */}
              <div
                style={{
                  marginTop: 20,
                  paddingTop: 16,
                  borderTop: '1px solid var(--border-subtle)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  flexWrap: 'wrap',
                }}
              >
                <PageGuide
                  pages={[
                    {
                      icon: Upload,
                      title: t('common:drag_upload_guide_title') ?? '拖拽附件上传指南',
                      steps: [
                        {
                          icon: Maximize2,
                          title: t('common:guide_detail_step1_title') ?? '拖拽到此面板',
                          description:
                            t('common:guide_detail_step1_desc') ??
                            '直接将文件从文件管理器拖入当前详情面板，即可为此对象添加附件。拖入时面板会高亮提示。',
                        },
                        {
                          icon: Paperclip,
                          title: t('common:guide_detail_step2_title') ?? '附件管理器',
                          description:
                            t('common:guide_detail_step2_desc') ??
                            '点击「附件」按钮打开附件管理器，也可将文件直接拖入管理器窗口进行批量上传。',
                        },
                      ],
                      helpLinks: [
                        {
                          title: t('common:guide_help_attachments') ?? '附件管理',
                          description:
                            t('common:guide_help_attachments_desc') ??
                            '附件的上传、下载、重命名与回收站管理',
                          href: '/help?id=attachments',
                        },
                      ],
                    },
                  ]}
                />
                <button
                  onClick={() => {
                    if (onHistory) {
                      onHistory();
                    } else {
                      setShowHistory(true);
                    }
                  }}
                  style={actionBtnStyle}
                  onMouseEnter={onActionBtnEnter}
                  onMouseLeave={onActionBtnLeave}
                >
                  <Clock size={ICON_SIZE.sm} /> {t('common:history')}
                </button>
                <button
                  onClick={() => {
                    if (onAttachments) {
                      onAttachments();
                    } else {
                      setShowAttachments(true);
                    }
                  }}
                  style={actionBtnStyle}
                  onMouseEnter={onActionBtnEnter}
                  onMouseLeave={onActionBtnLeave}
                >
                  <Paperclip size={ICON_SIZE.sm} /> {t('common:attachments')}
                </button>
                {onEdit && (
                  <button
                    onClick={onEdit}
                    style={actionBtnStyle}
                    onMouseEnter={onActionBtnEnter}
                    onMouseLeave={onActionBtnLeave}
                  >
                    <Pencil size={ICON_SIZE.sm} /> {t('common:edit')}
                  </button>
                )}
                <DeleteButton
                  onClick={() => {
                    if (onDelete) {
                      onDelete();
                    } else {
                      setConfirmDelete(true);
                    }
                  }}
                  title={t('common:delete')}
                >
                  {t('common:delete')}
                </DeleteButton>
              </div>
            </>
          </motion.div>
        )}
      </div>

      {confirmDelete && obj && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 3100,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'var(--bg-overlay)',
            backdropFilter: 'blur(4px)',
          }}
          onClick={() => setConfirmDelete(false)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 12,
              padding: '24px 28px',
              maxWidth: 360,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            <h3
              style={{ margin: '0 0 8px', fontSize: 'var(--text-section-title)', fontWeight: 600 }}
            >
              {t('common:object_delete_confirm_title')}
            </h3>
            <p
              style={{
                margin: '0 0 20px',
                fontSize: 'var(--text-body)',
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:object_delete_confirm_body', {
                name: obj.name.length > 28 ? obj.name.slice(0, 27) + '…' : obj.name,
              })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <Button variant="secondary" onClick={() => setConfirmDelete(false)}>
                {t('common:cancel')}
              </Button>
              <Button variant="danger-outline" onClick={handleDelete} disabled={deleting}>
                {t('common:delete')}
              </Button>
            </div>
          </div>
        </div>
      )}

      {showHistory && obj && (
        <HistoryViewer
          objectId={obj.id}
          objectName={obj.name}
          collectionType={obj.collectionType}
          onClose={() => setShowHistory(false)}
          passwordVerify={passwordVerify}
          getFieldSensitivity={getFieldSensitivity}
          isFieldDeprecated={isFieldDeprecated}
          getFieldName={getFieldName}
          fieldOrder={fieldOrder}
          zIndex={3100}
        />
      )}
      {showAttachments && obj && (
        <AttachmentViewer
          objectId={obj.id}
          onClose={() => setShowAttachments(false)}
          onCountChange={onAttachmentsChange}
          zIndex={3100}
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
