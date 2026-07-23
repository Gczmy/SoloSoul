import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { saveWithPause } from '@/lib/dialog';
import { Search, Download, X } from 'lucide-react';
import { resolveCollectionLabel } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settingsStore';
import { ICON_SIZE } from '@/lib/constants';
import { isUriPath, copyStagedFileToDest } from '@/lib/mobileFileTransfer';
import buttonStyles from '@/components/ui/Button.module.css';

interface AuditLogEntry {
  id: number;
  timestamp: string;
  actionType: string;
  entityType: string;
  entityId: string | null;
  entityName: string | null;
  performedBy: string;
  details: string | null;
}

/** All known entity types — used for filter buttons */
const ALL_ENTITY_TYPES = [
  'object',
  'page',
  'preference',
  'profile',
  'biometric',
  'template',
  'export',
  'import',
  'attachment',
  'trash_item',
  'llm',
  'auth',
  'file',
  'ocr_model',
];

function formatDetail(
  entry: AuditLogEntry,
  t: TFunction,
  customPages: import('@/stores/settingsStore').CustomPage[],
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

export function OperationLogPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t, i18n } = useTranslation(['settings', 'common']);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [entityTypeFilter, setEntityTypeFilter] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  const loadLogs = useCallback(async () => {
    setIsLoading(true);
    try {
      const entries = await invoke<AuditLogEntry[]>('log_get_recent', { limit: 200 });
      setLogs(entries);
    } catch (e) {
      onError(e, t('common:logs_load_failed'));
    } finally {
      setIsLoading(false);
    }
  }, [onError, t]);

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  const handleExport = async () => {
    try {
      const filePath = await saveWithPause({
        defaultPath: 'audit_log_export.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!filePath) return; // User cancelled

      // Android 保存对话框返回 content:// URI，Rust 只能写到应用私有目录，
      // 先导出到默认位置再用 plugin-fs 中转。
      if (isUriPath(filePath)) {
        const exportedPath = await invoke<string>('log_export', {});
        await copyStagedFileToDest(exportedPath, filePath);
        onSuccess(`${t('common:export')} → ${filePath}`);
      } else {
        const result = await invoke<string>('log_export', { exportPath: filePath });
        onSuccess(`${t('common:export')} → ${result}`);
      }
    } catch (e) {
      onError(e, t('common:logs_export_failed'));
    }
  };

  const filteredLogs = logs.filter((entry) => {
    if (entityTypeFilter && entry.entityType !== entityTypeFilter) return false;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      return (
        entry.actionType.toLowerCase().includes(q) ||
        entry.entityType.toLowerCase().includes(q) ||
        (entry.entityName?.toLowerCase().includes(q) ?? false) ||
        (entry.details?.toLowerCase().includes(q) ?? false)
      );
    }
    return true;
  });

  return (
    <AppShell title={t('settings:operation_log')} onBack={() => navigate('/settings')}>
      <PageContainer variant="wide" gap="default">
        {/* Row 1: Search + Export */}
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <div
            style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              border: '1px solid var(--border-subtle)',
              borderRadius: 8,
              padding: '0 10px',
            }}
          >
            <Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('settings:search_logs')}
              style={{
                flex: 1,
                border: 'none',
                outline: 'none',
                padding: '8px 4px',
                fontSize: 'var(--text-sm)',
                background: 'transparent',
                color: 'var(--text-primary)',
                fontFamily: 'inherit',
              }}
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                aria-label={t('common:clear')}
                tabIndex={-1}
                style={{
                  flexShrink: 0,
                  width: 22,
                  height: 22,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  border: 'none',
                  borderRadius: 4,
                  background: 'transparent',
                  color: 'var(--text-tertiary)',
                  cursor: 'pointer',
                  padding: 0,
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                  e.currentTarget.style.color = 'var(--text-tertiary)';
                }}
              >
                <X size={ICON_SIZE.sm} />
              </button>
            )}
          </div>
          <Button
            variant="secondary"
            size="sm"
            className={buttonStyles.hideLabelOnMobile}
            aria-label={t('settings:export_logs') || 'Export logs'}
            onClick={handleExport}
          >
            <Download size={ICON_SIZE.sm} />{' '}
            <span className={buttonStyles.label}>{t('settings:export_logs')}</span>
          </Button>
        </div>

        {/* Row 2: Entity type filter — workspace style */}
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
          {[null, ...ALL_ENTITY_TYPES].map((type) => {
            const isActive = entityTypeFilter === type;
            const label =
              type === null ? t('settings:all') : t(`settings:log.entity.${type}`, type);
            return (
              <button
                key={type ?? 'all'}
                onClick={() => setEntityTypeFilter(type === entityTypeFilter ? null : type)}
                onMouseEnter={(e) => {
                  if (!isActive) {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  }
                }}
                onMouseLeave={(e) => {
                  if (!isActive) {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }
                }}
                style={{
                  padding: '5px 12px',
                  borderRadius: 8,
                  border: isActive
                    ? '1px solid var(--accent-primary)'
                    : '1px solid var(--border-subtle)',
                  background: isActive
                    ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                    : 'var(--bg-toolbar)',
                  color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                  boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                  cursor: 'pointer',
                  fontSize: 'var(--text-sm)',
                  fontWeight: 500,
                  transition: 'all 0.15s ease',
                }}
              >
                {label}
              </button>
            );
          })}
        </div>

        {/* Log entries */}
        {isLoading ? (
          <LoadingPlaceholder variant="base" minHeight={200} />
        ) : filteredLogs.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: 40, color: 'var(--text-tertiary)' }}>
              <p style={{ fontSize: 'var(--text-sm)' }}>{t('settings:no_log_entries')}</p>
              <p style={{ fontSize: 'var(--text-caption)', marginTop: 4 }}>
                {searchQuery || entityTypeFilter
                  ? t('settings:adjust_filters')
                  : t('settings:logs_hint')}
              </p>
            </div>
          </Card>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
            {filteredLogs.map((entry) => (
              <Card key={entry.id}>
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
                        {entry.entityName && entry.entityType !== 'template'
                          ? `: ${entry.entityName}`
                          : ''}
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
                    {entry.details && formatDetail(entry, t, customPages) && (
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
                        {formatDetail(entry, t, customPages)}
                      </div>
                    )}
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </PageContainer>
    </AppShell>
  );
}
