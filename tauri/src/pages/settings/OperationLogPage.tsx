import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { Search, Download } from 'lucide-react';

interface LogEntry {
  timestamp: string;
  level: string;
  module: string;
  message: string;
  details?: string | null;
}

const LEVEL_COLORS: Record<string, string> = {
  info: 'var(--accent-primary, #3b82f6)',
  success: 'var(--accent-success, #22c55e)',
  warning: 'var(--accent-warning, #f59e0b)',
  error: 'var(--accent-danger, #ef4444)',
  debug: 'var(--text-tertiary, #9ca3af)',
};

export function OperationLogPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [levelFilter, setLevelFilter] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    loadLogs();
  }, []);

  const loadLogs = async () => {
    setIsLoading(true);
    try {
      const entries = await invoke<LogEntry[]>('log_get_recent', { limit: 200 });
      setLogs(entries);
    } catch (e) {
      onError(e, t('common:logs_load_failed'));
    } finally {
      setIsLoading(false);
    }
  };

  const handleExport = async () => {
    try {
      const path = await invoke<string>('log_export');
      onSuccess(`Exported to ${path}`);
    } catch (e) {
      onError(e, t('common:logs_export_failed'));
    }
  };

  const filteredLogs = logs.filter((entry) => {
    if (levelFilter && entry.level !== levelFilter) return false;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      return (
        entry.message.toLowerCase().includes(q) ||
        entry.module.toLowerCase().includes(q) ||
        (entry.details?.toLowerCase().includes(q) ?? false)
      );
    }
    return true;
  });

  const levels = [...new Set(logs.map((l) => l.level))];

  return (
    <AppShell title={t('settings:operation_log')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 720, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Toolbar */}
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
          <div style={{
            flex: 1, display: 'flex', alignItems: 'center', gap: 6,
            border: '1px solid var(--border-subtle)', borderRadius: 8,
            padding: '0 10px', minWidth: 200,
          }}>
            <Search size={14} style={{ color: 'var(--text-tertiary)' }} />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('settings:search_logs')}
              style={{
                flex: 1, border: 'none', outline: 'none', padding: '8px 4px',
                fontSize: 14, background: 'transparent', color: 'var(--text-primary)',
                fontFamily: 'inherit',
              }}
            />
          </div>

          <div style={{ display: 'flex', gap: 4 }}>
            <button
              onClick={() => setLevelFilter(null)}
              style={{
                padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                background: levelFilter === null ? 'var(--accent-primary)' : 'transparent',
                color: levelFilter === null ? 'white' : 'var(--text-primary)',
                cursor: 'pointer', fontSize: 12, fontWeight: 500,
              }}
            >
              {t('settings:all')}
            </button>
            {levels.map((level) => (
              <button
                key={level}
                onClick={() => setLevelFilter(level === levelFilter ? null : level)}
                style={{
                  padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                  background: levelFilter === level ? (LEVEL_COLORS[level] || 'var(--accent-primary)') : 'transparent',
                  color: levelFilter === level ? 'white' : 'var(--text-primary)',
                  cursor: 'pointer', fontSize: 12, fontWeight: 500,
                  textTransform: 'capitalize',
                }}
              >
                {level}
              </button>
            ))}
          </div>

          <Button variant="secondary" size="sm" onClick={handleExport}>
            <Download size={14} />
            {t('settings:export_logs')}
          </Button>
        </div>

        {/* Log entries */}
        {isLoading ? (
          <div style={{ textAlign: 'center', padding: 40, color: 'var(--text-tertiary)' }}>
            {t('settings:loading_logs')}
          </div>
        ) : filteredLogs.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: 40, color: 'var(--text-tertiary)' }}>
              <p style={{ fontSize: 14 }}>{t('settings:no_log_entries')}</p>
              <p style={{ fontSize: 12, marginTop: 4 }}>
                {searchQuery || levelFilter ? t('settings:adjust_filters') : t('settings:logs_hint')}
              </p>
            </div>
          </Card>
        ) : (
          filteredLogs.map((entry, i) => (
            <Card key={i}>
              <div style={{ display: 'flex', gap: 12, fontSize: 13 }}>
                <div style={{
                  width: 3, borderRadius: 2, flexShrink: 0,
                  backgroundColor: LEVEL_COLORS[entry.level] || 'var(--text-tertiary)',
                }} />

                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                    <span style={{
                      fontSize: 11, fontWeight: 600, padding: '1px 6px',
                      borderRadius: 4, textTransform: 'uppercase',
                      backgroundColor: (LEVEL_COLORS[entry.level] || 'var(--text-tertiary)') + '20',
                      color: LEVEL_COLORS[entry.level] || 'var(--text-tertiary)',
                    }}>
                      {entry.level}
                    </span>
                    <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                      {entry.module}
                    </span>
                    <span style={{ fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>
                      {new Date(entry.timestamp).toLocaleString()}
                    </span>
                  </div>
                  <p style={{ margin: 0, color: 'var(--text-primary)' }}>{entry.message}</p>
                  {entry.details && (
                    <p style={{
                      margin: '4px 0 0', fontSize: 12, color: 'var(--text-secondary)',
                      fontFamily: 'monospace', whiteSpace: 'pre-wrap',
                      backgroundColor: 'var(--bg-subtle, rgba(128,128,128,0.04))',
                      padding: '6px 8px', borderRadius: 4,
                    }}>
                      {entry.details}
                    </p>
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
