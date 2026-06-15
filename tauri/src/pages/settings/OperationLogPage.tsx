import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { Search, Download } from 'lucide-react';
import { resolveCollectionLabel } from '@/lib/pageLabels';
import { useSettingsStore } from '@/stores/settingsStore';

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
];

function formatDetail(
  entry: AuditLogEntry,
  t: TFunction,
  customPages: import('@/stores/settingsStore').CustomPage[],
): string {
  const raw = entry.details || '';

  // Normalize touch_id_unlock / face_id_unlock to biometric_unlock with explicit type
  const bioTypeFromAction =
    entry.actionType === 'touch_id_unlock'
      ? 'touchId'
      : entry.actionType === 'face_id_unlock'
        ? 'faceId'
        : null;
  const key = bioTypeFromAction
    ? 'settings:log.detail.biometric_unlock'
    : `settings:log.detail.${entry.actionType}`;

  const translated = t(key, { defaultValue: raw });
  if (translated === key || translated === raw) {
    return raw;
  }
  // Parse key=value pairs where values may contain spaces (e.g. objectName=My Passport)
  const re = /(\w+)=([^]*?)(?=(?:\s+\w+=|$))/g;
  const vars: Record<string, string | number> = {};
  let match;
  while ((match = re.exec(raw)) !== null) {
    const val = match[2].trim();
    vars[match[1]] = /^\d+$/.test(val) ? parseInt(val, 10) : val;
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
    if (vars.section)
      vars.section = resolveCollectionLabel(String(vars.section), customPages, t);
    // Translate unlock method for critical field access logs
    if (vars.method)
      vars.method = t(`settings:log.method.${vars.method}`, { defaultValue: String(vars.method) });
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

  useEffect(() => {
    loadLogs();
  }, []);

  const loadLogs = async () => {
    setIsLoading(true);
    try {
      const entries = await invoke<AuditLogEntry[]>('log_get_recent', { limit: 200 });
      setLogs(entries);
    } catch (e) {
      onError(e, t('common:logs_load_failed'));
    } finally {
      setIsLoading(false);
    }
  };

  const handleExport = async () => {
    try {
      const filePath = await save({
        defaultPath: 'audit_log_export.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!filePath) return; // User cancelled
      const result = await invoke<string>('log_export', { exportPath: filePath });
      onSuccess(`${t('common:export')} → ${result}`);
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
      <div
        style={{
          maxWidth: 720,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 12,
        }}
      >
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
            <Search size={14} style={{ color: 'var(--text-tertiary)' }} />
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
                fontSize: 14,
                background: 'transparent',
                color: 'var(--text-primary)',
                fontFamily: 'inherit',
              }}
            />
          </div>
          <Button variant="secondary" size="sm" onClick={handleExport}>
            <Download size={14} />
            {t('settings:export_logs')}
          </Button>
        </div>

        {/* Row 2: Entity type filter */}
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
          <button
            onClick={() => setEntityTypeFilter(null)}
            style={{
              padding: '5px 10px',
              borderRadius: 6,
              border: '1px solid var(--border-subtle)',
              background: entityTypeFilter === null ? 'var(--accent-primary)' : 'transparent',
              color: entityTypeFilter === null ? 'white' : 'var(--text-primary)',
              cursor: 'pointer',
              fontSize: 12,
              fontWeight: 500,
            }}
          >
            {t('settings:all')}
          </button>
          {ALL_ENTITY_TYPES.map((type) => (
            <button
              key={type}
              onClick={() => setEntityTypeFilter(type === entityTypeFilter ? null : type)}
              style={{
                padding: '5px 10px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: entityTypeFilter === type ? 'var(--accent-primary)' : 'transparent',
                color: entityTypeFilter === type ? 'white' : 'var(--text-primary)',
                cursor: 'pointer',
                fontSize: 12,
                fontWeight: 500,
              }}
            >
              {t(`settings:log.entity.${type}`, type)}
            </button>
          ))}
        </div>

        {/* Log entries */}
        {isLoading ? (
          <LoadingPlaceholder variant="base" minHeight={200} />
        ) : filteredLogs.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: 40, color: 'var(--text-tertiary)' }}>
              <p style={{ fontSize: 14 }}>{t('settings:no_log_entries')}</p>
              <p style={{ fontSize: 12, marginTop: 4 }}>
                {searchQuery || entityTypeFilter
                  ? t('settings:adjust_filters')
                  : t('settings:logs_hint')}
              </p>
            </div>
          </Card>
        ) : (
          filteredLogs.map((entry) => (
            <Card key={entry.id}>
              <div style={{ display: 'flex', gap: 12, fontSize: 13 }}>
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
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                    <span
                      style={{
                        fontSize: 11,
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
                        fontSize: 11,
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
                          fontSize: 10,
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
                      style={{ fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 'auto' }}
                    >
                      {new Date(entry.timestamp).toLocaleString(i18n.language)}
                    </span>
                  </div>
                  {entry.details && formatDetail(entry, t, customPages) && (
                    <div
                      style={{
                        margin: '4px 0 0',
                        fontSize: 12,
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
          ))
        )}
      </div>
    </AppShell>
  );
}
