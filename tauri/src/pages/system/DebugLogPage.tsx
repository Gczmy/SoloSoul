import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';

import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { Bug, Download, RefreshCw } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { isUriPath, copyStagedFileToDest } from '@/lib/mobileFileTransfer';


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

export function DebugLogPage() {
  const navigate = useNavigate();
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [levelFilter] = useState<string>('all');
  const [isLoading, setIsLoading] = useState(true);
  const { t } = useTranslation(['settings', 'common']);

  const loadLogs = async () => {
    setIsLoading(true);
    try {
      const entries = await invoke<AuditLogEntry[]>('log_get_recent', { limit: 200 });
      setLogs(entries);
    } catch {
      setLogs([]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadLogs();
  }, []);

  const handleExport = async () => {
    try {
      const filePath = await save({
        defaultPath: 'debug_log_export.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!filePath) return;

      // Android 保存对话框返回 content:// URI，Rust 只能写到应用私有目录，
      // 先导出到默认位置再用 plugin-fs 中转。
      if (isUriPath(filePath)) {
        const exportedPath = await invoke<string>('log_export', {});
        await copyStagedFileToDest(exportedPath, filePath);
      } else {
        await invoke<string>('log_export', { exportPath: filePath });
      }
    } catch {
      // silent
    }
  };

  const filteredLogs =
    levelFilter === 'all' ? logs : logs.filter((l) => l.entityType === levelFilter);

  return (
    <AppShell title={t('settings:debug_log')} onBack={() => navigate('/settings')}>
      <PageContainer variant="wide" gap="default">
        {/* Toolbar */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
          <button
            onClick={loadLogs}
            style={{
              padding: '6px 10px',
              borderRadius: 6,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-primary)',
            }}
          >
            <RefreshCw size={ICON_SIZE.sm} /> {t('settings:refresh')}
          </button>

          <button
            onClick={handleExport}
            style={{
              padding: '6px 10px',
              borderRadius: 6,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-primary)',
            }}
          >
            <Download size={ICON_SIZE.sm} /> {t('settings:export')}
          </button>

          <span
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              marginLeft: 'auto',
            }}
          >
            {filteredLogs.length} {t('settings:entries_count')}
          </span>
        </div>

        {/* Log list */}
        <Card>
          {isLoading ? (
            <LoadingPlaceholder variant="elevated" minHeight={200} />
          ) : filteredLogs.length === 0 ? (
            <div
              style={{
                textAlign: 'center',
                padding: 32,
                color: 'var(--text-tertiary)',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              <Bug size={ICON_SIZE['2xl']} style={{ margin: '0 auto 8px', opacity: 0.4 }} />
              {t('settings:no_log_entries_debug')}
            </div>
          ) : (
            <div
              style={{
                fontFamily: 'var(--font-mono, "SF Mono", Monaco, monospace)',
                fontSize: 'var(--text-caption)',
                lineHeight: 1.6,
              }}
            >
              {filteredLogs.map((log) => (
                <div
                  key={log.id}
                  style={{
                    display: 'flex',
                    gap: 8,
                    padding: '2px 4px',
                    background: log.actionType.includes('delete')
                      ? 'rgba(220, 38, 38, 0.06)'
                      : log.actionType.includes('create')
                        ? 'rgba(34,197,94,0.06)'
                        : 'transparent',
                    borderRadius: 2,
                  }}
                >
                  <span
                    style={{ color: 'var(--text-tertiary)', whiteSpace: 'nowrap', minWidth: 80 }}
                  >
                    {log.timestamp?.split('T')[1]?.split('.')[0] || ''}
                  </span>
                  <span
                    style={{
                      minWidth: 48,
                      fontWeight: 600,
                      color: log.actionType.includes('delete')
                        ? '#dc2626'
                        : log.actionType.includes('create')
                          ? 'var(--accent-success)'
                          : 'var(--accent-primary)',
                    }}
                  >
                    {log.actionType.toUpperCase()}
                  </span>
                  <span
                    style={{
                      color: 'var(--text-secondary)',
                      flex: 1,
                      minWidth: 0,
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                    }}
                  >
                    {log.entityType}
                    {log.entityName ? `: ${log.entityName}` : ''}
                    {log.details ? ` - ${log.details}` : ''}
                  </span>
                </div>
              ))}
            </div>
          )}
        </Card>
      </PageContainer>
    </AppShell>
  );
}
