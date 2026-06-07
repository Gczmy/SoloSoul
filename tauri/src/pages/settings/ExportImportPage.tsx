import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';

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
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const [tab, setTab] = useState<'export' | 'import'>('export');
  const [scope, setScope] = useState<ScopeNode[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [password, setPassword] = useState('');
  const [exportHint, setExportHint] = useState('');
  const [isExporting, setIsExporting] = useState(false);

  // Import state
  const [importPath, setImportPath] = useState('');
  const [savePath, setSavePath] = useState<string | null>(null);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [importPw, setImportPw] = useState('');
  const [isImporting, setIsImporting] = useState(false);
  const [isPreviewing, setIsPreviewing] = useState(false);

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
          savePath,
          passwordHint: exportHint || null,
        },
      });
      onSuccess(t('settings:export_success'));
    } catch (e) { onError(e, t('common:export_failed'));
    } finally { setIsExporting(false); }
  };

  const handlePreviewImport = async () => {
    if (!importPath || isPreviewing) return;
    setIsPreviewing(true);
    try {
      const preview = await invoke<ImportPreview>('import_preview_package', { filePath: importPath });
      // Sanitize: ensure profileNames is always an array
      // Debug: log raw response to console
      console.log('import_preview_package raw:', JSON.stringify(preview));
      const safe = { ...preview, profileNames: Array.isArray(preview.profileNames) ? preview.profileNames : (preview.profileNames || []) };
      setImportPreview(safe);
    } catch (e) { onError(e, t('common:preview_failed')); }
    finally { setIsPreviewing(false); }
  };

  const handleImport = async () => {
    if (!importPath || !importPw) return;
    setIsImporting(true);
    try {
      const count = await invoke<number>('import_execute', { filePath: importPath, password: importPw });
      onSuccess(t('settings:import_success', { count: count }));
      setImportPreview(null);
      invoke<ScopeNode[]>('export_get_scope_tree').then(setScope).catch(() => {});
    } catch (e) { onError(e, t('common:import_failed'));
    } finally { setIsImporting(false); }
  };

  return (
    <AppShell title={t('settings:export_import')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <div style={{ display: 'flex', gap: 0, borderRadius: 8, overflow: 'hidden', border: '1px solid var(--border-subtle)' }}>
          {(['export', 'import'] as const).map((tabKey) => (
            <button key={tabKey} onClick={() => setTab(tabKey)} style={{
              flex: 1, padding: '10px', border: 'none', cursor: 'pointer',
              background: tab === tabKey ? 'var(--accent-primary)' : 'transparent',
              color: tab === tabKey ? 'white' : 'var(--text-primary)',
              fontSize: 14, fontWeight: 500,
            }}>{tabKey === 'export' ? t('settings:export') : t('settings:import')}</button>
          ))}
        </div>

        {tab === 'export' && (
          <>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
              {t('export_desc', { ns: 'settings', defaultValue: 'Select profiles to export as an encrypted .solosoul file.' })}
            </p>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>{t('settings:profiles')}</h3>
              {scope.length === 0 ? (
                <p style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>{t('common:no_data')}</p>
              ) : (
                scope.map((node) => (
                  <label key={node.id} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 0', cursor: 'pointer' }}>
                    <input type="checkbox" checked={selectedIds.has(node.id)}
                      onChange={() => toggleId(node.id)}
                      style={{ accentColor: 'var(--accent-primary)' }} />
                    <span style={{ fontSize: 14 }}>{node.name}</span>
                    <span style={{ fontSize: 12, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>
                      {t('common:object_count', { n: node.itemCount })}
                    </span>
                  </label>
                ))
              )}
            </Card>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>{t('common:export_path')}</h3>
              <div style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 8 }}>
                {savePath || t('settings:no_file_selected')}
              </div>
              <Button variant="secondary" size="sm" onClick={async () => {
                const fp = await save({
                  filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
                  defaultPath: `solosoul_export_${Date.now()}.solosoul`,
                });
                if (fp) setSavePath(fp);
              }}>
                {t('common:browse')}
              </Button>
            </Card>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>{t('settings:encryption')}</h3>
              <SecurePasswordInput value={password}
                onChange={(v) => setPassword(v)}
                placeholder={t('common:password_placeholder')} />
              <div style={{ marginTop: 8 }}>
                <input type="text" value={exportHint}
                  onChange={(e) => setExportHint(e.target.value)}
                  placeholder={t('common:password_hint')}
                  style={{ width: '100%', padding: '10px 14px', fontSize: 14, border: '1px solid var(--border-subtle)', borderRadius: 8, background: 'var(--bg-elevated)', color: 'var(--text-primary)', fontFamily: 'inherit', outline: 'none' }} />
              </div>
            </Card>
            <Button onClick={handleExport} loading={isExporting}
              disabled={selectedIds.size === 0 || !password || !savePath}>
              {t('settings:export_selected')} ({selectedIds.size})
            </Button>
          </>
        )}

        {tab === 'import' && (
          <>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
              {t('settings:import_desc')}
            </p>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>{t('settings:select_file')}</h3>
              <div style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 8 }}>
                {importPath || t('settings:no_file_selected')}
              </div>
              <Button size="sm" onClick={async () => {
                const selected = await open({
                  filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
                  multiple: false,
                });
                if (selected) {
                  setImportPath(selected as string);
                  setImportPreview(null);
                }
              }}>
                {t('settings:select_file')}
              </Button>
              {importPath && (
                <div style={{ marginTop: 8 }}>
                  <Button size="sm" onClick={handlePreviewImport} loading={isPreviewing} disabled={isPreviewing || !!importPreview}>
                    {t('settings:preview')}
                  </Button>
                </div>
              )}
            </Card>

            {importPreview ? (
              <Card>
                <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>{t('settings:import_preview')}</h3>
                <div style={{ fontSize: 13, display: 'flex', flexDirection: 'column', gap: 4 }}>
                  <p>{t('settings:export_time')}: {importPreview.exportTime || t('settings:unknown')}</p>
                  <p>{t('settings:profiles_count')}: {importPreview.profileCount || 0}</p>
                  {(importPreview.profileNames || []).length > 0 && (
                    <ul style={{ margin: '4px 0 0 16px', fontSize: 12, color: 'var(--text-secondary)' }}>
                      {importPreview.profileNames.map((name) => (
                        <li key={name} style={{ marginBottom: 2 }}>{name}</li>
                      ))}
                    </ul>
                  )}
                </div>
                <div style={{ marginTop: 12 }}>
                  <SecurePasswordInput value={importPw}
                    onChange={(v) => setImportPw(v)}
                    placeholder={t('common:password_placeholder')} />
                </div>
                <div style={{ marginTop: 8 }}>
                  <Button onClick={handleImport} loading={isImporting} disabled={!importPw}>
                    {t('settings:import_success', { count: importPreview.profileCount || 0 })}
                  </Button>
                </div>
              </Card>
            ) : null}
          </>
        )}
      </div>
    </AppShell>
  );
}
