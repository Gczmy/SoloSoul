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
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';

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
  fieldOrder?: string[]
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
  const { maskValue, isRevealed, reveal } = useRevealState();
  const [fetchedObj, setFetchedObj] = useState<ObjectData | null>(null);
  const [loading, setLoading] = useState(!object && !!objectId);
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [showAttachments, setShowAttachments] = useState(false);

  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<((ok: boolean) => void) | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  const obj = useMemo(() => object || fetchedObj, [object, fetchedObj]);

  useEffect(() => {
    loadTemplates().catch(() => {});
  }, [loadTemplates]);

  useEffect(() => {
    if (!accountId) return;
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>('biometric_check_availability', {
      accountId,
    })
      .then((r) => setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }))
      .catch(() => {});
    invoke<Array<{ id: string; passwordHint?: string }>>('list_accounts')
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

  const verifyVaultPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        return await invoke<boolean>('verify_password', { accountId, password });
      } catch {
        return false;
      }
    },
    [accountId]
  );

  const passwordVerify = useCallback(async (): Promise<boolean> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', { accountId, location: 'critical_data_access', action: 'unlock' });
      pwResolveRef.current?.(true);
      return true;
    } catch {
      return false;
    }
  }, [accountId]);

  const handleRevealField = useCallback(
    async (fieldId: string, sens: SensitivityLevel) => {
      if (sens === 'critical') {
        const ok = await passwordVerify();
        if (ok) reveal(fieldId);
      } else {
        reveal(fieldId);
      }
    },
    [passwordVerify, reveal]
  );

  const getFieldSensitivity = (fieldKey: string): SensitivityLevel => {
    const prop = templates
      .find((tpl) => tpl.id === obj?.templateId)
      ?.properties.find((p) => p.id === fieldKey);
    return (prop?.sensitivityLevel as SensitivityLevel) || 'public';
  };

  const isFieldDeprecated = (fieldKey: string): boolean => {
    const prop = templates
      .find((tpl) => tpl.id === obj?.templateId)
      ?.properties.find((p) => p.id === fieldKey);
    return !!prop?.deprecatedAt;
  };

  const getFieldName = (fieldKey: string): string => {
    const prop = templates
      .find((tpl) => tpl.id === obj?.templateId)
      ?.properties.find((p) => p.id === fieldKey);
    return prop?.name || fieldKey;
  };

  const handleCopy = async (value: string, key: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopiedField(key);
      setTimeout(() => setCopiedField(null), 1500);
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

  const resolveCollectionLabel = (collectionType: string) => {
    if (['identity', 'travel', 'financial', 'professional'].includes(collectionType)) {
      return t(`navigation:${collectionType}`);
    }
    return collectionType;
  };

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
          }}
        >
          {loading || !obj ? (
            <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 32 }}>
              {t('common:loading')}
            </p>
          ) : (
            <>
              {/* Header */}
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 20 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <PAGE_ICON_MAP.custom size={24} />
                  <div>
                    <h2 style={{ fontSize: 18, fontWeight: 700, margin: 0 }}>{obj.name}</h2>
                    <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {resolveCollectionLabel(obj.collectionType)} · {t('common:created')}:{' '}
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
                  }}
                >
                  <X size={20} />
                </button>
              </div>

              <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 16 }} />

              {/* Fields */}
              {fields.length === 0 ? (
                <p style={{ fontSize: 13, color: 'var(--text-tertiary)', textAlign: 'center', padding: '16px 0' }}>
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
                          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 2 }}>
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
                            {deprecated && <DeprecatedBadge />}
                          </div>
                          <div
                            style={{
                              fontSize: 14,
                              color: needsReveal && !revealed ? 'var(--text-tertiary)' : 'var(--text-primary)',
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
                                handleRevealField(fieldId, sens);
                              }}
                              style={{
                                padding: '4px 10px',
                                borderRadius: 6,
                                border: '1px solid var(--border-subtle)',
                                background: sens === 'critical' ? 'rgba(220,38,38,0.06)' : 'transparent',
                                cursor: 'pointer',
                                fontSize: 11,
                                color: sens === 'critical' ? '#dc2626' : 'var(--text-tertiary)',
                                display: 'flex',
                                alignItems: 'center',
                                gap: 4,
                              }}
                            >
                              {sens === 'critical' ? <Lock size={12} /> : <Eye size={12} />}{' '}
                              {sens === 'critical' ? t('common:unlock') : t('common:reveal')}
                            </button>
                          )}
                          <button
                            onClick={() => handleCopy(revealed ? f.value : maskValue(f.value, fieldId, sens), f.key)}
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
                >
                  <Paperclip size={14} /> {t('common:attachments')}
                </button>
                {onEdit && (
                  <button onClick={onEdit} style={actionBtnStyle}>
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
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('common:object_delete_confirm_body', { name: obj.name })}
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
          pwResolveRef.current?.(false);
        }}
        onVerify={async (password) => {
          const ok = await verifyVaultPassword(password);
          if (ok) pwResolveRef.current?.(true);
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
