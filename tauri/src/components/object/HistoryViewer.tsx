import { useState, useEffect, useLayoutEffect, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Clock, ChevronLeft, ChevronRight, X } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import type { PropertyType } from '@/types/template';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import { ProtectedFieldValue } from '@/components/ui/ProtectedFieldValue';
import {
  fieldPresentationIdentity,
  fieldPresentationPolicy,
  strongestSensitivity,
} from '@/lib/fieldPresentationPolicy';
import { asFieldRecord } from '@/lib/fieldSensitivity';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { useAuthStore } from '@/stores/authStore';
import { resolveCollectionLabel } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import { logger } from '@/lib/logger';
import { ICON_SIZE } from '@/lib/constants';
import { ValueContainer } from '@/components/ui/ValueContainer';
import { flattenPropertyEntries, type DynamicChildItem } from '@/lib/propertyFlatten';

import type { SnapshotEntry } from '@/types/history';

export type FlattenedField =
  | {
      kind: 'field';
      key: string;
      value: string;
      label?: string;
      sensitivity?: SensitivityLevel;
      type?: PropertyType;
    }
  | {
      kind: 'dynamicGroup';
      key: string;
      label?: string;
      sensitivity?: SensitivityLevel;
      type?: PropertyType;
      children: DynamicChildItem[];
    };

// P024: 收敛至共享核心 flattenPropertyEntries——历史快照需要保留 __fields 中的
// __ 前缀 key（如 __dynamic_group__）且 dynamic_group 保持分组结构。
export function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
): FlattenedField[] {
  // P024-R1: 历史快照以快照内 __fields 名称为准（快照本身就是该时刻的字段定义），
  // 显式开启 injectFieldLabels；普通对象展示（objectDetailUtils/WorkspaceObjectCard）
  // 保持旧语义不注入，回退消费端当前模板名。
  return flattenPropertyEntries(props, fieldOrder, undefined, {
    keepMetaKeys: true,
    flattenDynamicGroups: false,
    injectFieldLabels: true,
    preserveChildSensitivity: true,
  }).map((e) =>
    e.kind === 'field'
      ? {
          kind: 'field',
          key: e.key,
          value: e.value,
          label: e.label,
          type: e.type as PropertyType | undefined,
        }
      : {
          kind: 'dynamicGroup',
          key: e.key,
          label: e.label,
          type: e.type as PropertyType | undefined,
          children: e.children,
        },
  );
}

type Verification = HistoryViewerProps['passwordVerify'];
type HistoryRequest = ReturnType<ReturnType<typeof createSessionRequests>['begin']>;
type CriticalAccess = (
  name: string,
  method: Awaited<ReturnType<Verification>>['method'],
  request: HistoryRequest,
) => Promise<void>;

/** 读取与审计也受实例寿命约束，卡片卸载时同步拒绝迟到响应。 */
function useHistoryRequests() {
  const [requests] = useState(createSessionRequests);
  useLayoutEffect(() => () => requests.invalidate(), [requests]);
  return requests;
}

function HistoryField({
  accountId,
  objectId,
  snapshotId,
  fieldId,
  label,
  value,
  sensitivity,
  type,
  deprecated,
  child,
  showBadge = true,
  verifyPassword,
  onCriticalAccess,
}: {
  accountId?: string;
  objectId: string;
  snapshotId: string;
  fieldId: string;
  label: string;
  value: string;
  sensitivity: SensitivityLevel;
  type?: PropertyType;
  deprecated?: boolean;
  child?: boolean;
  showBadge?: boolean;
  verifyPassword: Verification;
  onCriticalAccess: CriticalAccess;
}) {
  const { t } = useTranslation(['common']);
  const requests = useHistoryRequests();
  const policy = fieldPresentationPolicy({
    fieldId,
    definition: { sensitivityLevel: sensitivity },
  });
  const authorize = async () => {
    const request = requests.begin('verify', accountId);
    if (!request.isCurrent()) return false;
    if (policy.requiresVerification) {
      const result = await verifyPassword();
      if (!result.ok || !request.isCurrent()) return false;
      await onCriticalAccess(label, result.method, request);
    }
    return request.isCurrent();
  };
  return (
    <ProtectedFieldValue
      accountId={accountId}
      objectId={objectId}
      fieldId={fieldId}
      contentVersion={snapshotId}
      value={value}
      policy={policy}
      authorize={authorize}
    >
      {(control) => {
        const seconds = Math.max(0, Math.ceil(control.remainingMs / 1000));
        const countdownTitle = t('common:reveal_countdown_title', {
          seconds,
          defaultValue: `Auto-hides in ${seconds}s`,
        });
        const revealLabel = t('common:click_to_reveal', { defaultValue: 'Click to reveal' });
        return (
          <div
            style={{
              display: 'flex',
              flexWrap: 'wrap',
              alignItems: 'flex-start',
              gap: 8,
              marginLeft: child ? 16 : undefined,
              fontSize: 'var(--text-caption)',
              padding: '6px 8px',
              borderRadius: 6,
              background: 'var(--bg-toolbar)',
              border: '1px solid var(--border-subtle)',
              opacity: deprecated ? 0.7 : 1,
            }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 4,
                minWidth: child ? 74 : 90,
                flex: '0 0 auto',
              }}
            >
              <FieldTypeIcon type={type || 'text'} />
              <span
                style={{
                  fontWeight: 500,
                  color: 'var(--text-secondary)',
                  textDecoration: deprecated ? 'line-through' : 'none',
                }}
              >
                {label}
              </span>
              {showBadge && <SensitivityBadge level={sensitivity} />}
              {deprecated && <DeprecatedBadge />}
              {control.revealed && (
                <span
                  title={countdownTitle}
                  aria-label={countdownTitle}
                  data-testid="history-reveal-countdown"
                  style={{
                    flexShrink: 0,
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 3,
                    minWidth: '3.5em',
                    fontSize: 'var(--text-badge)',
                    color: 'var(--text-tertiary)',
                    fontVariantNumeric: 'tabular-nums',
                    whiteSpace: 'nowrap',
                  }}
                >
                  <Clock size={12} aria-hidden="true" />
                  {t('common:reveal_countdown', { seconds, defaultValue: `${seconds}s` })}
                </span>
              )}
            </div>
            <ValueContainer value={control.displayValue}>
              {policy.concealed && !control.revealed ? (
                <button
                  type="button"
                  onClick={() => void control.reveal()}
                  title={revealLabel}
                  aria-label={revealLabel}
                  style={{
                    cursor: 'pointer',
                    userSelect: 'none',
                    background: 'var(--bg-subtle, rgba(128,128,128,0.15))',
                    border: 'none',
                    borderRadius: 2,
                    padding: '0 2px',
                    color: 'var(--text-primary)',
                    font: 'inherit',
                  }}
                >
                  {control.displayValue}
                </button>
              ) : (
                <span>{control.displayValue}</span>
              )}
            </ValueContainer>
          </div>
        );
      }}
    </ProtectedFieldValue>
  );
}

function SnapshotCard({
  snap,
  accountId,
  objectId,
  index,
  total,
  verifyPassword,
  onCriticalAccess,
}: {
  snap: SnapshotEntry;
  accountId?: string;
  objectId: string;
  index: number;
  total: number;
  verifyPassword: Verification;
  onCriticalAccess: CriticalAccess;
}) {
  const [snapData, setSnapData] = useState<Record<string, unknown> | null>(null);
  const requests = useHistoryRequests();
  const { t } = useTranslation(['common', 'editor']);
  useEffect(() => {
    const request = requests.begin('data', accountId);
    request
      .invoke<Record<string, unknown> | null>('snapshot_get_data', { snapshotId: snap.id })
      .then((data) => {
        if (request.isCurrent()) setSnapData(data);
      })
      .catch((err) => {
        if (request.isCurrent()) logger.warn('[HistoryViewer] snapshot_get_data failed:', err);
      });
    return () => requests.invalidate('data');
  }, [requests, accountId, snap.id]);

  const rawProps = asFieldRecord(snapData?.properties);
  const labels = asFieldRecord(snapData?.propertyLabels);
  const fieldDefs = asFieldRecord(rawProps?.__fields);
  // 只采用快照定义顺序；缺少定义的旧快照保留其 properties 顺序。
  const fields = useMemo(
    () =>
      flattenProperties(rawProps, [
        ...new Set([...Object.keys(fieldDefs ?? {}), ...Object.keys(rawProps ?? {})]),
      ]),
    [rawProps, fieldDefs],
  );
  const snapName = snapData?.name == null ? '' : String(snapData.name);
  const tags = Array.isArray(snapData?.tags)
    ? snapData.tags.filter((tag): tag is string => typeof tag === 'string')
    : [];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      <div style={{ display: 'flex', alignItems: 'flex-start' }}>
        <SnapshotVersionBadge index={index} total={total} />
        <div
          style={{
            marginLeft: 'auto',
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-primary)',
            fontWeight: 500,
            overflowWrap: 'break-word',
            wordBreak: 'break-word',
            textAlign: 'right',
            maxWidth: '70%',
          }}
        >
          {snapName}
        </div>
      </div>
      {fields.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginTop: 4 }}>
          {fields.map((field) => {
            const definition = asFieldRecord(fieldDefs?.[field.key]);
            const sensitivity = fieldPresentationPolicy({
              fieldId: field.key,
              propertyLabels: labels,
              definition,
            }).sensitivity;
            const deprecated = !!definition?.deprecatedAt;
            const name = field.label ?? field.key;
            const label =
              field.key === '__dynamic_group__' || name === '__dynamic_group__'
                ? t('editor:field_types.dynamic_group', { defaultValue: '动态字段组' })
                : name;
            const renderField = (
              fieldId: string,
              fieldLabel: string,
              value: string,
              level: SensitivityLevel,
              type?: PropertyType,
              child = false,
            ) => (
              <HistoryField
                key={fieldPresentationIdentity(accountId, objectId, fieldId, [
                  snap.id,
                  value,
                  level,
                  fieldLabel,
                ])}
                accountId={accountId}
                objectId={objectId}
                snapshotId={snap.id}
                fieldId={fieldId}
                label={fieldLabel}
                value={value}
                sensitivity={level}
                type={type}
                deprecated={deprecated}
                child={child}
                showBadge={!child}
                verifyPassword={verifyPassword}
                onCriticalAccess={onCriticalAccess}
              />
            );
            if (field.kind === 'field')
              return renderField(field.key, label, field.value, sensitivity, field.type);
            const children = field.children.map((child) => ({
              ...child,
              level: fieldPresentationPolicy({
                fieldId: child.id ?? child.label,
                definition: { sensitivityLevel: child.sensitivityLevel },
                parent: sensitivity,
              }).sensitivity,
            }));
            return (
              <div key={field.key} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                    fontSize: 'var(--text-caption)',
                    padding: '6px 8px',
                    borderRadius: 6,
                    background: 'var(--bg-toolbar)',
                    border: '1px solid var(--border-subtle)',
                    opacity: deprecated ? 0.7 : 1,
                  }}
                >
                  <FieldTypeIcon type={field.type || 'dynamic_group'} />
                  <span
                    style={{
                      fontWeight: 500,
                      color: 'var(--text-secondary)',
                      textDecoration: deprecated ? 'line-through' : 'none',
                    }}
                  >
                    {label}
                  </span>
                  <SensitivityBadge
                    level={strongestSensitivity([
                      sensitivity,
                      ...children.map((child) => child.level),
                    ])}
                  />
                  {deprecated && <DeprecatedBadge />}
                </div>
                {children.map((child, index) =>
                  renderField(
                    `${field.key}.${child.id ?? index}`,
                    child.label,
                    child.value,
                    child.level,
                    child.type as PropertyType,
                    true,
                  ),
                )}
              </div>
            );
          })}
        </div>
      )}
      {tags.length > 0 && (
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
          {tags.map((tag) => (
            <span
              key={tag}
              style={{
                padding: '1px 7px',
                borderRadius: 10,
                fontSize: 'var(--text-badge)',
                background: 'rgba(91,124,153,0.08)',
                color: 'var(--accent-primary)',
                fontWeight: 500,
              }}
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
export interface HistoryViewerProps {
  objectId: string;
  objectName?: string;
  typeId?: string;
  onClose: () => void;
  passwordVerify: () => Promise<{
    ok: boolean;
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';
  }>;
  /** 兼容旧调用方；当前模板元数据不参与历史展示。 */
  getFieldSensitivity: (fieldKey: string) => SensitivityLevel;
  isFieldDeprecated: (fieldKey: string) => boolean;
  getFieldName: (fieldKey: string) => string;
  fieldOrder?: string[];
  zIndex?: number;
}

export function HistoryViewer(props: HistoryViewerProps) {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const [sessionVersion, setSessionVersion] = useState(0);
  useLayoutEffect(
    () => onRequestSessionChange(() => setSessionVersion((version) => version + 1)),
    [],
  );
  return (
    <HistoryViewerSession
      key={JSON.stringify([accountId, props.objectId, sessionVersion])}
      {...props}
      accountId={accountId}
    />
  );
}

function HistoryViewerSession({
  accountId,
  objectId,
  objectName,
  typeId,
  onClose,
  passwordVerify,
  zIndex = 2000,
}: HistoryViewerProps & { accountId?: string }) {
  const requests = useHistoryRequests();
  const [snapshots, setSnapshots] = useState<SnapshotEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentIdx, setCurrentIdx] = useState(0);
  const [animDir, setAnimDir] = useState<'left' | 'right' | null>(null);
  const navTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { t } = useTranslation(['common', 'editor', 'navigation']);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const showToast = useUiStore((s) => s.showToast);

  const resolveCollectionLabelLocal = (typeId: string) =>
    resolveCollectionLabel(typeId, customPages, t);

  const writeCriticalAccessLog = async (
    fieldName: string,
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin',
    request: HistoryRequest,
  ) => {
    if (!objectName) return;
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
    const pageLabel = typeId ? resolveCollectionLabelLocal(typeId) : '';
    const details = `objectName=${objectName} page=${pageLabel} fieldName=${fieldName}`;
    try {
      await request.invoke('log_write', {
        request: {
          actionType,
          entityType,
          entityId: objectId,
          entityName: null,
          details,
        },
      });
    } catch {
      // best effort
    }
  };

  useEffect(() => {
    const request = requests.begin('list', accountId);
    request
      .invoke<SnapshotEntry[]>('snapshot_list', { objectId: objectId })
      .then((data) => {
        if (request.isCurrent()) setSnapshots(data);
      })
      .catch((err) => {
        if (!request.isCurrent()) return;
        // P059: 补齐 .catch，失败时给出提示而非 unhandled rejection
        showToast({
          type: 'error',
          message: `${t('common:history_load_failed', 'Failed to load history')}: ${err}`,
        });
      })
      .finally(() => {
        if (request.isCurrent()) setLoading(false);
      });
    return () => requests.invalidate('list');
    // showToast/t 为稳定引用，仅需在 objectId 变化时重新加载
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [objectId, accountId, requests]);

  useLayoutEffect(() => {
    return () => {
      if (navTimeoutRef.current) {
        clearTimeout(navTimeoutRef.current);
      }
    };
  }, []);

  const goPrev = () => {
    if (!animDir && currentIdx < snapshots.length - 1) {
      setAnimDir('right');
      navTimeoutRef.current = setTimeout(() => {
        setCurrentIdx((i) => i + 1);
        setAnimDir(null);
      }, 150);
    }
  };
  const goNext = () => {
    if (!animDir && currentIdx > 0) {
      setAnimDir('left');
      navTimeoutRef.current = setTimeout(() => {
        setCurrentIdx((i) => i - 1);
        setAnimDir(null);
      }, 150);
    }
  };

  const snap = snapshots[currentIdx];
  const total = snapshots.length;
  const isOldest = currentIdx >= total - 1;
  const isLatest = currentIdx <= 0;

  return (
    <div
      data-macos-glass-backdrop
      style={{
        position: 'fixed',
        inset: 0,
        zIndex,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)',
        backdropFilter: 'blur(6px)',
      }}
      onClick={onClose}
    >
      {!loading && (
        <div
          data-macos-glass="panel"
          onClick={(e) => e.stopPropagation()}
          style={{
            position: 'relative',
            width: 460,
            maxHeight: '80vh',
            overflowY: 'auto',
            display: 'flex',
            flexDirection: 'column',
            background: 'var(--bg-elevated)',
            borderRadius: 16,
            boxShadow: '0 24px 80px rgba(0,0,0,0.25)',
            border: '1px solid var(--border-subtle)',
            animation: 'fadeIn 0.2s ease-out',
            transform:
              animDir === 'left'
                ? 'perspective(1200px) rotateY(-8deg)'
                : animDir === 'right'
                  ? 'perspective(1200px) rotateY(8deg)'
                  : 'perspective(1200px) rotateY(0)',
            transition: 'transform 0.15s ease',
            transformOrigin:
              animDir === 'left' ? 'left center' : animDir === 'right' ? 'right center' : 'center',
          }}
        >
          {/* Header */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '14px 18px',
              borderBottom: '1px solid var(--border-subtle)',
            }}
          >
            <div
              style={{
                fontSize: 'var(--text-body-sm)',
                fontWeight: 600,
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              <Clock size={ICON_SIZE.sm} /> {t('common:history')}
              <span
                style={{
                  fontSize: 'var(--text-badge)',
                  color: 'var(--text-tertiary)',
                  fontWeight: 400,
                }}
              >
                {loading ? '' : `${currentIdx + 1} / ${total}`}
              </span>
            </div>
            <div style={{ display: 'flex', gap: 6 }}>
              <BadgeIconButton
                Icon={ChevronLeft}
                onClick={goPrev}
                title={t('common:previous', { defaultValue: 'Previous' })}
                disabled={isOldest || loading}
                iconSize={ICON_SIZE.md}
              />
              <BadgeIconButton
                Icon={ChevronRight}
                onClick={goNext}
                title={t('common:next', { defaultValue: 'Next' })}
                disabled={isLatest || loading}
                iconSize={ICON_SIZE.md}
              />
              <BadgeIconButton
                Icon={X}
                onClick={onClose}
                title={t('common:close', { defaultValue: 'Close' })}
                iconSize={ICON_SIZE.md}
              />
            </div>
          </div>
          {/* Content */}
          <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
            {!snap ? (
              <div
                style={{
                  textAlign: 'center',
                  padding: 48,
                  color: 'var(--text-secondary)',
                  fontSize: 'var(--text-body)',
                }}
              >
                {t('common:no_history')}
              </div>
            ) : animDir ? null : (
              <SnapshotCard
                key={snap.id}
                accountId={accountId}
                objectId={objectId}
                snap={snap}
                index={currentIdx}
                total={total}
                verifyPassword={passwordVerify}
                onCriticalAccess={writeCriticalAccessLog}
              />
            )}
          </div>
          {/* Footer */}
          <div
            style={{
              padding: '10px 18px',
              borderTop: '1px solid var(--border-subtle)',
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
            }}
          >
            {snap &&
              (() => {
                const triggerLabel = t(`common:trigger_${snap.triggeredBy}` as const, {
                  defaultValue: snap.triggeredBy,
                });
                return `${t('common:version')} #${total - currentIdx} · ${new Date(snap.timestamp).toLocaleString()} · ${triggerLabel}`;
              })()}
          </div>
        </div>
      )}
    </div>
  );
}
