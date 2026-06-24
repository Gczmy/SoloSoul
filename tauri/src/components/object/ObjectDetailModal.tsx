import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { X, Clock, Paperclip, Pencil, Trash2, Lock, Eye } from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useObjectStore, type ObjectData, type ObjectSummary } from '@/stores/objectStore';
import { useRevealState } from '@/hooks/useRevealState';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { PluginBadge } from '@/components/template/PluginBadge';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { resolveCollectionLabel } from '@/lib/pageLabels';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useSettingsStore } from '@/stores/settingsStore';
import type { TemplateProperty } from '@/types/template';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';

interface ObjectDetailModalProps {
  /** 已加载的对象摘要/完整数据。与 objectId 二选一，优先使用此值。 */
  object?: ObjectSummary | ObjectData;
  /** 若未提供 object，则通过 objectId 自动拉取完整对象数据。 */
  objectId?: string;
  onClose: () => void;
  onHistory?: () => void;
  onAttachments?: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
}

function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
): { key: string; value: string }[] {
  if (!props) return [];
  const entries: { key: string; value: string }[] = [];
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith('__')) continue;
    if (v === null || v === undefined || v === '') continue;
    if (typeof v === 'string') {
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
  onClose,
  onHistory,
  onAttachments,
  onEdit,
  onDelete,
}: ObjectDetailModalProps) {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const { templates, loadTemplates } = useTemplateStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const { maskValue, isRevealed, reveal } = useRevealState();
  const [fetchedObj, setFetchedObj] = useState<ObjectData | null>(null);
  const [loading, setLoading] = useState(!object && !!objectId);
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
    ((result: { ok: boolean; method: 'password' | 'touchId' | 'faceId' }) => void) | null
  >(null);
  const pendingRevealRef = useRef<{ fieldId: string; fieldName: string } | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  const obj = useMemo(() => object || fetchedObj, [object, fetchedObj]);
  const { ref: detailDragRef, dragState: detailDragState } = useDragToAttach(obj?.id || null);

  useEffect(() => {
    loadTemplates().catch(() => {});
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
      .catch(() => {});
    invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
      .then((accounts) => {
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      })
      .catch(() => {});
  }, [accountId]);

  useEffect(() => {
    if (object || !objectId || !accountId) {
      if (!object && !objectId) setLoading(false);
      return;
    }
    setLoading(true);
    useObjectStore
      .getState()
      .getObject(accountId, objectId)
      .then(() => {
        setFetchedObj(useObjectStore.getState().currentObject);
      })
      .catch(() => setFetchedObj(null))
      .finally(() => setLoading(false));
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
    method: 'password' | 'touchId' | 'faceId';
  }> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  const resolveCollectionLabelLocal = useCallback((collectionType: string) =>
    resolveCollectionLabel(collectionType, customPages, t),
  [customPages, t]);


  const writeCriticalAccessLog = useCallback(
    async (method: 'password' | 'touchId' | 'faceId') => {
      if (!accountId || !obj || !pendingRevealRef.current) return;
      const actionType =
        method === 'password'
          ? 'critical_field_login'
          : method === 'touchId'
            ? 'critical_field_touch_id'
            : 'critical_field_face_id';
      const entityType = method === 'password' ? 'auth' : 'biometric';
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
      const method = (bioAvailable.biometryType as 'touchId' | 'faceId') || 'touchId';
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

  const getFieldProperty = (fieldKey: string): TemplateProperty | undefined => {
    return fieldMap.get(fieldKey);
  };

  const getFieldSensitivity = (fieldKey: string): SensitivityLevel => {
    return (getFieldProperty(fieldKey)?.sensitivityLevel as SensitivityLevel) || 'public';
  };

  const isFieldDeprecated = (fieldKey: string): boolean => {
    return !!getFieldProperty(fieldKey)?.deprecatedAt;
  };

  const getFieldName = (fieldKey: string): string => {
    return getFieldProperty(fieldKey)?.name || fieldKey;
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
  const ObjectDetailIcon = detailTpl?.iconId ? resolveCustomIcon(detailTpl.iconId) : PAGE_ICON_MAP.custom;
  const fieldOrder = templates.find((t) => t.id === obj?.templateId)?.properties.map((p) => p.id);
  const fields = flattenProperties(obj?.properties, fieldOrder);

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
    fontSize: 13,
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
  const onDeleteBtnEnter = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background = 'rgba(231,76,60,0.1)';
    e.currentTarget.style.borderColor = 'rgba(231,76,60,0.3)';
    e.currentTarget.style.color = '#e74c3c';
  };
  const onDeleteBtnLeave = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background = 'var(--bg-toolbar)';
    e.currentTarget.style.borderColor = 'var(--border-subtle)';
    e.currentTarget.style.color = '#e74c3c';
  };

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
          background: 'rgba(0,0,0,0.35)',
          backdropFilter: 'blur(4px)',
        }}
        onClick={onClose}
      >
        <div
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
          {loading || !obj ? (
            <LoadingPlaceholder variant="elevated" minHeight={160} />
          ) : (
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
                    <ObjectDetailIcon size={24} />
                  </span>
                  <div>
                    <h2 style={{ fontSize: 18, fontWeight: 700, margin: 0, overflowWrap: 'break-word', wordBreak: 'break-word' }}>{obj.name}</h2>
                    <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {resolveCollectionLabelLocal(obj.collectionType)}
                      {obj.contractTypeId && (
                        <span style={{ marginLeft: 4, display: 'inline-flex', verticalAlign: 'middle' }}>
                          <PluginBadge contractTypeId={obj.contractTypeId} size="sm" variant="full" />
                        </span>
                      )}
                      {' · '}{t('common:created')}:{' '}
                      {obj.createdAt?.slice(0, 10) || '—'} · {t('common:updated')}:{' '}
                      {obj.updatedAt?.slice(0, 10) || '—'}
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
                    e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'transparent';
                    e.currentTarget.style.color = 'var(--text-tertiary)';
                  }}
                >
                  <X size={20} />
                </button>
              </div>

              <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 16 }} />

              {/* Fields */}
              {fields.length === 0 ? (
                <p
                  style={{
                    fontSize: 13,
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
                    const fieldId = `${obj.collectionType}.${f.key}`;
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
                                fontSize: 12,
                                fontWeight: 600,
                                color: 'var(--text-secondary)',
                                textDecoration: deprecated ? 'line-through' : 'none',
                              }}
                            >
                              {getFieldName(f.key)}
                            </span>
                            <SensitivityBadge level={sens} />
                            {obj.contractTypeId && <PluginBadge contractTypeId={obj.contractTypeId} size="sm" variant="full" />}
                            {deprecated && <DeprecatedBadge />}
                          </div>
                          <div
                            style={{
                              fontSize: 14,
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
                                fontSize: 11,
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
                                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
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
                              {sens === 'critical' ? <Lock size={12} /> : <Eye size={12} />}{' '}
                              {sens === 'critical' ? t('common:unlock') : t('common:reveal')}
                            </button>
                          )}
                          <button
                            onClick={() =>
                              handleCopy(
                                revealed ? f.value : maskValue(f.value, fieldId, sens),
                                f.key,
                              )
                            }
                            style={{
                              padding: '4px 10px',
                              borderRadius: 6,
                              border: '1px solid var(--border-subtle)',
                              background: 'transparent',
                              cursor: 'pointer',
                              fontSize: 11,
                              color: copiedField === f.key ? '#27ae60' : 'var(--text-tertiary)',
                              display: 'flex',
                              alignItems: 'center',
                              gap: 4,
                              transition: 'all 0.15s ease',
                            }}
                            onMouseEnter={(e) => {
                              if (copiedField !== f.key) {
                                e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                                e.currentTarget.style.color = 'var(--accent-primary)';
                                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                              }
                            }}
                            onMouseLeave={(e) => {
                              if (copiedField !== f.key) {
                                e.currentTarget.style.background = 'transparent';
                                e.currentTarget.style.color = 'var(--text-tertiary)';
                                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              }
                            }}
                          >
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
                        fontSize: 11,
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
                  gap: 8,
                  flexWrap: 'wrap',
                }}
              >
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
                  <Clock size={14} /> {t('common:history')}
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
                  <Paperclip size={14} /> {t('common:attachments')}
                </button>
                {onEdit && (
                  <button
                    onClick={onEdit}
                    style={actionBtnStyle}
                    onMouseEnter={onActionBtnEnter}
                    onMouseLeave={onActionBtnLeave}
                  >
                    <Pencil size={14} /> {t('common:edit')}
                  </button>
                )}
                <button
                  onClick={() => {
                    if (onDelete) {
                      onDelete();
                    } else {
                      setConfirmDelete(true);
                    }
                  }}
                  style={{
                    ...actionBtnStyle,
                    color: '#e74c3c',
                  }}
                  onMouseEnter={onDeleteBtnEnter}
                  onMouseLeave={onDeleteBtnLeave}
                >
                  <Trash2 size={14} /> {t('common:delete')}
                </button>
              </div>
            </>
          )}
        </div>
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
            background: 'rgba(0,0,0,0.4)',
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
            <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
              {t('common:object_delete_confirm_title')}
            </h3>
            <p
              style={{
                margin: '0 0 20px',
                fontSize: 14,
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:object_delete_confirm_body', { name: obj.name.length > 28 ? obj.name.slice(0, 27) + '…' : obj.name })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <Button variant="secondary" onClick={() => setConfirmDelete(false)}>
                {t('common:cancel')}
              </Button>
              <button
                onClick={handleDelete}
                disabled={deleting}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: 'none',
                  background: '#e74c3c',
                  color: 'white',
                  fontSize: 13,
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = '#c0392b';
                  e.currentTarget.style.boxShadow = '0 2px 8px rgba(231,76,60,0.35)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = '#e74c3c';
                  e.currentTarget.style.boxShadow = 'none';
                }}
              >
                {t('common:delete')}
              </button>
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
        biometricType={bioAvailable.available ? bioAvailable.biometryType : undefined}
        onBiometric={bioAvailable.available ? handleBiometricUnlock : undefined}
      />
    </>
  );
}
