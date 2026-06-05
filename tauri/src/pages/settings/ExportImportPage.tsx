import { useState, useEffect } from 'react';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';

interface ScopeNode {
  id: string; name: string; nodeType: string;
  children: ScopeNode[]; itemCount: number; attachmentCount: number;
}

interface ImportPreview {
  filePath: string;
  exportTime: string | null;
  profileCount: number;
  profileNames: string[];
}

export function ExportImportPage() {
  const { onError, onSuccess } = useToastError();
  const [tab, setTab] = useState<'export' | 'import'>('export');
  const [scope, setScope] = useState<ScopeNode[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [password, setPassword] = useState('');
  const [isExporting, setIsExporting] = useState(false);

  // Import state
  const [importPath, setImportPath] = useState('');
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [importPw, setImportPw] = useState('');
  const [isImporting, setIsImporting] = useState(false);

  useEffect(() => {
    invoke<ScopeNode[]>('export_get_scope_tree').then(setScope).catch(() => {});
  }, []);

  const toggleId = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const handleExport = async () => {
    if (selectedIds.size === 0 || !password) return;
    setIsExporting(true);
    try {
      const path = await invoke<string>('export_execute', {
        req: {
          profileIds: Array.from(selectedIds),
          includePreferences: true, includeAuditLog: false,
          password,
          savePath: `~/Downloads/solosoul_export_${Date.now()}`,
        },
      });
      onSuccess(`Exported to ${path}`);
    } catch (e) { onError(e, 'Export failed');
    } finally { setIsExporting(false); }
  };

  const handlePreviewImport = async () => {
    if (!importPath) return;
    try {
      const preview = await invoke<ImportPreview>('import_preview_package', { filePath: importPath });
      setImportPreview(preview);
    } catch (e) { onError(e, 'Preview failed'); }
  };

  const handleImport = async () => {
    if (!importPath || !importPw) return;
    setIsImporting(true);
    try {
      const count = await invoke<number>('import_execute', { filePath: importPath, password: importPw });
      onSuccess(`Imported ${count} profile(s)`);
      setImportPreview(null);
      invoke<ScopeNode[]>('export_get_scope_tree').then(setScope).catch(() => {});
    } catch (e) { onError(e, 'Import failed');
    } finally { setIsImporting(false); }
  };

  return (
    <AppShell title="Export & Import" onBack={() => window.history.back()}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <div style={{ display: 'flex', gap: 0, borderRadius: 8, overflow: 'hidden', border: '1px solid var(--border-subtle)' }}>
          {(['export', 'import'] as const).map((t) => (
            <button key={t} onClick={() => setTab(t)} style={{
              flex: 1, padding: '10px', border: 'none', cursor: 'pointer',
              background: tab === t ? 'var(--accent-primary)' : 'transparent',
              color: tab === t ? 'white' : 'var(--text-primary)',
              fontSize: 14, fontWeight: 500, textTransform: 'capitalize',
            }}>{t}</button>
          ))}
        </div>

        {tab === 'export' && (
          <>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
              Select profiles to export as an encrypted .solosoul file.
            </p>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>Profiles</h3>
              {scope.length === 0 ? (
                <p style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>No profiles found</p>
              ) : (
                scope.map((node) => (
                  <label key={node.id} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 0', cursor: 'pointer' }}>
                    <input type="checkbox" checked={selectedIds.has(node.id)}
                      onChange={() => toggleId(node.id)}
                      style={{ accentColor: 'var(--accent-primary)' }} />
                    <span style={{ fontSize: 14 }}>{node.name}</span>
                    <span style={{ fontSize: 12, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>
                      {node.itemCount} item{node.itemCount !== 1 ? 's' : ''}
                    </span>
                  </label>
                ))
              )}
            </Card>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>Encryption</h3>
              <SecurePasswordInput value={password}
                onChange={(v) => setPassword(v)}
                placeholder="Enter export password" />
            </Card>
            <Button onClick={handleExport} loading={isExporting}
              disabled={selectedIds.size === 0 || !password}>
              Export Selected ({selectedIds.size})
            </Button>
          </>
        )}

        {tab === 'import' && (
          <>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
              Import profiles from a .solosoul file.
            </p>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>Select File</h3>
              <Input value={importPath}
                onChange={(e) => { setImportPath(e.target.value); setImportPreview(null); }}
                placeholder="Path to .solosoul file" />
              <div style={{ marginTop: 8 }}>
                <Button size="sm" onClick={handlePreviewImport} disabled={!importPath}>
                  Preview
                </Button>
              </div>
            </Card>

            {importPreview && (
              <Card>
                <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>Import Preview</h3>
                <div style={{ fontSize: 13, display: 'flex', flexDirection: 'column', gap: 4 }}>
                  <p>Export time: {importPreview.exportTime || 'Unknown'}</p>
                  <p>Profiles: {importPreview.profileCount}</p>
                  <ul style={{ margin: '4px 0 0 16px', fontSize: 12, color: 'var(--text-secondary)' }}>
                    {importPreview.profileNames.map((name) => (
                      <li key={name} style={{ marginBottom: 2 }}>{name}</li>
                    ))}
                  </ul>
                </div>
                <div style={{ marginTop: 12 }}>
                  <SecurePasswordInput value={importPw}
                    onChange={(v) => setImportPw(v)}
                    placeholder="Export password" />
                </div>
                <div style={{ marginTop: 8 }}>
                  <Button onClick={handleImport} loading={isImporting} disabled={!importPw}>
                    Import {importPreview.profileCount} Profile(s)
                  </Button>
                </div>
              </Card>
            )}
          </>
        )}
      </div>
    </AppShell>
  );
}
