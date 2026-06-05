import { useState, useEffect } from 'react';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { invoke } from '@tauri-apps/api/core';
import { Bug, Download, RefreshCw } from 'lucide-react';

interface LogEntry {
  id: number;
  timestamp: string;
  level: string;
  module: string;
  message: string;
}

const LEVELS = ['error', 'warn', 'info', 'debug', 'trace'] as const;

export function DebugLogPage() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<string>('all');
  const [isLoading, setIsLoading] = useState(true);

  const loadLogs = async () => {
    setIsLoading(true);
    try {
      const entries = await invoke<LogEntry[]>('log_get_recent', { limit: 200 });
      setLogs(entries);
    } catch {
      // Fallback to empty
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
      const path = await invoke<string>('log_export', { format: 'text' });
      // For now copy to clipboard as fallback
      await navigator.clipboard.writeText(
        filteredLogs.map((l) => `[${l.timestamp}] [${l.level}] ${l.message}`).join('\n')
      );
    } catch {
      // silent
    }
  };

  const filteredLogs = levelFilter === 'all'
    ? logs
    : logs.filter((l) => l.level === levelFilter);

  return (
    <AppShell title="Debug Log" onBack={() => window.history.back()}>
      <div style={{ maxWidth: 720, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 12 }}>

        {/* Toolbar */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
          <select
            value={levelFilter}
            onChange={(e) => setLevelFilter(e.target.value)}
            style={{
              padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
              fontSize: 13, background: 'var(--bg-elevated)', color: 'var(--text-primary)',
            }}
          >
            <option value="all">All Levels</option>
            {LEVELS.map((l) => (
              <option key={l} value={l}>{l.toUpperCase()}</option>
            ))}
          </select>

          <button
            onClick={loadLogs}
            style={{
              padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4,
              fontSize: 13, color: 'var(--text-primary)',
            }}
          >
            <RefreshCw size={14} /> Refresh
          </button>

          <button
            onClick={handleExport}
            style={{
              padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4,
              fontSize: 13, color: 'var(--text-primary)',
            }}
          >
            <Download size={14} /> Export
          </button>

          <span style={{ fontSize: 12, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>
            {filteredLogs.length} entries
          </span>
        </div>

        {/* Log list */}
        <Card>
          {isLoading ? (
            <div style={{ textAlign: 'center', padding: 32, color: 'var(--text-tertiary)', fontSize: 13 }}>
              Loading logs...
            </div>
          ) : filteredLogs.length === 0 ? (
            <div style={{ textAlign: 'center', padding: 32, color: 'var(--text-tertiary)', fontSize: 13 }}>
              <Bug size={24} style={{ margin: '0 auto 8px', opacity: 0.4 }} />
              No log entries found
            </div>
          ) : (
            <div style={{ fontFamily: 'var(--font-mono, "SF Mono", Monaco, monospace)', fontSize: 12, lineHeight: 1.6 }}>
              {filteredLogs.map((log, i) => (
                <div
                  key={log.id || i}
                  style={{
                    display: 'flex', gap: 8, padding: '2px 4px',
                    background: log.level === 'error' ? 'rgba(220, 38, 38, 0.06)' :
                                log.level === 'warn' ? 'rgba(196, 146, 92, 0.06)' : 'transparent',
                    borderRadius: 2,
                  }}
                >
                  <span style={{ color: 'var(--text-tertiary)', whiteSpace: 'nowrap', minWidth: 80 }}>
                    {log.timestamp?.split('T')[1]?.split('.')[0] || ''}
                  </span>
                  <span style={{
                    minWidth: 48, fontWeight: 600,
                    color: log.level === 'error' ? '#dc2626' :
                           log.level === 'warn' ? '#c4925c' :
                           log.level === 'info' ? 'var(--accent-primary)' : 'var(--text-tertiary)',
                  }}>
                    {log.level?.toUpperCase()}
                  </span>
                  <span style={{ color: 'var(--text-secondary)', flex: 1 }}>{log.message}</span>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </AppShell>
  );
}
