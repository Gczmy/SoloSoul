import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { formatBytes } from '@/lib/utils';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useToastError } from '@/hooks/useToastError';
import { FolderOpen, FileText, Upload, Search } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

interface ScannedFile {
  path: string;
  name: string;
  size: number;
  ext: string;
}

const SUPPORTED_EXTS = new Set([
  'pdf',
  'txt',
  'md',
  'json',
  'csv',
  'xml',
  'png',
  'jpg',
  'jpeg',
  'gif',
  'webp',
  'bmp',
  'doc',
  'docx',
  'xls',
  'xlsx',
  'ppt',
  'pptx',
]);

export function ScanLocalPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { createObject } = useObjectStore();
  const { onError, onSuccess } = useToastError();

  const [files, setFiles] = useState<ScannedFile[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [selectedDir, setSelectedDir] = useState('');
  const [importing, setImporting] = useState<Set<string>>(new Set());

  const handleSelectDir = async () => {
    try {
      const dir = await open({
        directory: true,
        multiple: false,
        title: 'Select directory to scan',
      });
      if (dir && typeof dir === 'string') {
        setSelectedDir(dir);
        setIsScanning(true);
        const result = await invoke<ScannedFile[]>('fs_scan_directory', { path: dir });
        setFiles(result.filter((f) => SUPPORTED_EXTS.has(f.ext.toLowerCase())));
      }
    } catch (e) {
      onError(e, 'Failed to scan directory');
    } finally {
      setIsScanning(false);
    }
  };

  const handleImport = async (file: ScannedFile) => {
    if (!accountId) return;
    setImporting((prev) => new Set(prev).add(file.path));
    try {
      await createObject({
        accountId,
        name: file.name,
        collectionType: 'document',
        properties: {
          sourcePath: file.path,
          fileSize: file.size,
          fileExt: file.ext,
          importedAt: new Date().toISOString(),
        },
      });
      onSuccess(`Imported: ${file.name}`);
    } catch (e) {
      onError(e, `Failed to import: ${file.name}`);
    } finally {
      setImporting((prev) => {
        const next = new Set(prev);
        next.delete(file.path);
        return next;
      });
    }
  };

  const handleImportAll = async () => {
    const results = await Promise.allSettled(files.map((file) => handleImport(file)));
    const failures = results.filter((r) => r.status === 'rejected');
    if (failures.length > 0) {
      console.warn(`ImportAll: ${failures.length}/${files.length} files failed`);
    }
  };

  return (
    <AppShell
      title={t('settings:local_import', { defaultValue: 'Local Import' })}
      onBack={() => navigate(-1)}
    >
      <PageContainer variant="medium" gap="default">
        {/* Directory picker */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div
              style={{
                width: 44,
                height: 44,
                borderRadius: 10,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: 'rgba(91,124,153,0.1)',
              }}
            >
              <FolderOpen size={ICON_SIZE['2xl']} style={{ color: 'var(--accent-primary)' }} />
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>Select Directory</div>
              <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                {selectedDir || 'Choose a folder to scan for documents'}
              </div>
            </div>
            <Button onClick={handleSelectDir} loading={isScanning}>
              <Search size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> Scan
            </Button>
          </div>
        </Card>

        {/* File list */}
        {files.length > 0 && (
          <>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {files.length} file(s) found
              </span>
              <Button size="sm" variant="secondary" onClick={handleImportAll}>
                <Upload size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> Import All
              </Button>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
              {files.map((file) => (
                <Card key={file.path}>
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                      <FileText size={ICON_SIZE.xl} style={{ color: 'var(--text-tertiary)' }} />
                      <div>
                        <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                          {file.name}
                        </div>
                        <div
                          style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}
                        >
                          {file.ext.toUpperCase()} · {formatBytes(file.size)}
                        </div>
                      </div>
                    </div>
                    <Button
                      size="sm"
                      onClick={() => handleImport(file)}
                      loading={importing.has(file.path)}
                    >
                      Import
                    </Button>
                  </div>
                </Card>
              ))}
            </div>
          </>
        )}

        {!isScanning && files.length === 0 && selectedDir && (
          <Card>
            <p
              style={{
                textAlign: 'center',
                color: 'var(--text-secondary)',
                padding: 24,
                fontSize: 'var(--text-sm)',
              }}
            >
              No supported files found in this directory.
            </p>
          </Card>
        )}
      </PageContainer>
    </AppShell>
  );
}
