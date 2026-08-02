import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { resolveCollectionLabel } from '@/lib/utils';
import type { CustomPage } from '@/stores/settingsStore';

export interface AuditLogEntry {
  id: number;
  timestamp: string;
  actionType: string;
  entityType: string;
  entityId: string | null;
  entityName: string | null;
  performedBy: string;
  details: string | null;
}

function formatDetail(
  entry: AuditLogEntry,
  t: TFunction,
  customPages: CustomPage[],
): string {
  const raw = entry.details || '';

  // 优先解析后端已结构化的 JSON details（key=value 被规范化成对象），
  // 解析失败时退回到旧的 key=value 正则解析。
  const vars: Record<string, string | number | boolean> = {};
  let parsedObject = false;
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      Object.assign(vars, parsed);
      parsedObject = true;
    }
  } catch {
    // ignore
  }
  if (!parsedObject) {
    // Parse key=value pairs where values may contain spaces (e.g. objectName=My Passport)
    const re = /(\w+)=([^]*?)(?=(?:\s+\w+=|$))/g;
    let match;
    while ((match = re.exec(raw)) !== null) {
      const val = match[2].trim();
      vars[match[1]] = /^\d+$/.test(val) ? parseInt(val, 10) : val;
    }
  }

  // Normalize touch_id_unlock / face_id_unlock to biometric_unlock with explicit type
  const bioTypeFromAction =
    entry.actionType === 'touch_id_unlock'
      ? 'touchId'
      : entry.actionType === 'face_id_unlock'
        ? 'faceId'
        : entry.actionType === 'windows_hello_unlock'
          ? 'windowsHello'
          : null;
  // page_restore 有两种形态：直接恢复（带 count）和级联恢复（来自对象恢复）
  const isPageRestoreCascaded =
    entry.actionType === 'page_restore' && vars.cascadedFromObject === true;

  const key = bioTypeFromAction
    ? 'settings:log.detail.biometric_unlock'
    : isPageRestoreCascaded
      ? 'settings:log.detail.page_restore_cascaded'
      : `settings:log.detail.${entry.actionType}`;

  const translated = t(key, { defaultValue: raw });
  if (translated === key || translated === raw) {
    return raw;
  }
  if (entry.entityName) {
    vars.name = entry.entityName;
  }
  if (bioTypeFromAction) {
    vars.type = t(`settings:log.biometric_type.${bioTypeFromAction}`);
  } else if (vars.type) {
    vars.type = t(`settings:log.biometric_type.${vars.type}`, { defaultValue: String(vars.type) });
  }
  if (Object.keys(vars).length > 0) {
    if (vars.was_conflict === 'true') vars.was_conflict = t('settings:log.conflict_renamed');
    else if (vars.was_conflict === 'false') vars.was_conflict = t('settings:log.conflict_none');
    // Translate location and action codes to human-readable i18n
    if (vars.location)
      vars.location = t(`settings:log.location.${vars.location}`, {
        defaultValue: String(vars.location),
      });
    if (vars.action)
      vars.action = t(`settings:log.action_name.${vars.action}`, {
        defaultValue: String(vars.action),
      });
    // Resolve section to human-readable page/section name (built-in or custom page)
    if (vars.section) vars.section = resolveCollectionLabel(String(vars.section), customPages, t);
    // Translate unlock method for critical field access logs
    if (vars.method)
      vars.method = t(`settings:log.method.${vars.method}`, { defaultValue: String(vars.method) });
    // Translate import strategy (skipExisting / overwrite / keepBoth)
    if (vars.strategy) {
      const strategyKey = `settings:strategy_${vars.strategy}`;
      vars.strategy = t(strategyKey, { defaultValue: String(vars.strategy) });
    }
    return t(key, { defaultValue: raw, ...vars });
  }
  return t(key, { defaultValue: raw, reason: raw });
}

interface OperationLogCardProps {
  entry: AuditLogEntry;
  customPages: CustomPage[];
}

/**
 * 单条审计日志卡片（P218：memo 化）。
 *
 * 默认浅比较 props：`entry` 是 `logs` 状态数组中的稳定引用——搜索击键只重建
 * `filteredLogs`（filter 结果数组），各 `entry` 引用不变，卡片即跳过重渲染；
 * `customPages` 来自 settingsStore 选择器，仅设置变更时换引用。t/i18n 经内部
 * useTranslation 获取，语言切换仍会经 i18next 订阅触发重渲染（context 订阅
 * 不受 memo 短路影响）。
 */
export const OperationLogCard = memo(function OperationLogCard({
  entry,
  customPages,
}: OperationLogCardProps) {
  const { t, i18n } = useTranslation(['settings', 'common']);
  const detail = entry.details ? formatDetail(entry, t, customPages) : null;

  return (
    <Card>
      <div style={{ display: 'flex', gap: 12, fontSize: 'var(--text-body-sm)' }}>
        <div
          style={{
            width: 3,
            borderRadius: 2,
            flexShrink: 0,
            backgroundColor: entry.actionType.includes('delete')
              ? 'var(--accent-danger, #ef4444)'
              : entry.actionType.includes('create')
                ? 'var(--accent-success, #22c55e)'
                : 'var(--accent-primary, #3b82f6)',
          }}
        />

        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              marginBottom: 4,
              flexWrap: 'wrap',
            }}
          >
            <span
              style={{
                fontSize: 'var(--text-badge)',
                fontWeight: 600,
                padding: '1px 6px',
                borderRadius: 4,
                backgroundColor: entry.actionType.includes('delete')
                  ? 'rgba(239,68,68,0.12)'
                  : entry.actionType.includes('create')
                    ? 'rgba(34,197,94,0.12)'
                    : 'rgba(59,130,246,0.12)',
                color: entry.actionType.includes('delete')
                  ? 'var(--accent-danger, #ef4444)'
                  : entry.actionType.includes('create')
                    ? 'var(--accent-success, #22c55e)'
                    : 'var(--accent-primary, #3b82f6)',
              }}
            >
              {t(
                `settings:log.action.${entry.actionType}`,
                entry.actionType
                  .replace(/_/g, ' ')
                  .replace(/\b\w/g, (c) => c.toUpperCase()),
              )}
            </span>
            <span
              style={{
                fontSize: 'var(--text-badge)',
                fontWeight: 600,
                padding: '1px 6px',
                borderRadius: 4,
                backgroundColor:
                  entry.entityType === 'page'
                    ? 'rgba(139, 92, 246, 0.12)'
                    : entry.entityType === 'object'
                      ? 'rgba(34, 197, 94, 0.12)'
                      : 'var(--bg-subtle, rgba(128,128,128,0.08))',
                color:
                  entry.entityType === 'page'
                    ? '#8B5CF6'
                    : entry.entityType === 'object'
                      ? '#22c55e'
                      : 'var(--text-secondary)',
                wordBreak: 'break-word',
                minWidth: 0,
              }}
            >
              {t(`settings:log.entity.${entry.entityType}`, entry.entityType)}
              {entry.entityName && entry.entityType !== 'template' ? `: ${entry.entityName}` : ''}
            </span>
            {entry.performedBy === 'system' && (
              <span
                style={{
                  fontSize: 'var(--text-badge)',
                  padding: '1px 4px',
                  borderRadius: 3,
                  backgroundColor: 'var(--bg-subtle, rgba(128,128,128,0.08))',
                  color: 'var(--text-tertiary)',
                }}
              >
                {t('settings:log.performed_by_system')}
              </span>
            )}
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
                marginLeft: 'auto',
              }}
            >
              {new Date(entry.timestamp).toLocaleString(i18n.language)}
            </span>
          </div>
          {detail && (
            <div
              style={{
                margin: '4px 0 0',
                fontSize: 'var(--text-caption)',
                color: 'var(--text-secondary)',
                fontFamily: 'monospace',
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
                backgroundColor: 'var(--bg-subtle, rgba(128,128,128,0.04))',
                padding: '6px 8px',
                borderRadius: 4,
              }}
            >
              {detail}
            </div>
          )}
        </div>
      </div>
    </Card>
  );
});
