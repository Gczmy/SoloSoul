import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useToastError } from '@/hooks/useToastError';
import { Scan, FileText, Upload } from 'lucide-react';

interface OcrField {
  label: string;
  value: string;
  confidence: number;
}
interface OcrResult {
  text: string;
  confidence: number;
  fields: OcrField[];
}

export function OcrPage() {
  const navigate = useNavigate();
  useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { createObject } = useObjectStore();
  const { onError, onSuccess } = useToastError();

  const [filePath, setFilePath] = useState('');
  const [result, setResult] = useState<OcrResult | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  const handleSelectImage = async () => {
    try {
      const path = await open({
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }],
        multiple: false,
        title: 'Select image to scan',
      });
      if (path && typeof path === 'string') {
        setFilePath(path);
        setIsScanning(true);
        const res = await invoke<OcrResult>('ocr_scan_image', { filePath: path });
        setResult(res);
      }
    } catch (e) {
      onError(e, 'OCR scan failed');
    } finally {
      setIsScanning(false);
    }
  };

  const handleImportAsObject = async () => {
    if (!accountId || !result) return;
    setIsImporting(true);
    try {
      const props: Record<string, unknown> = { ocrText: result.text };
      for (const field of result.fields) {
        props[field.label.toLowerCase().replace(/\s+/g, '_')] = field.value;
      }
      await createObject({
        accountId,
        name: filePath.split('/').pop() || 'Scanned Document',
        collectionType: 'document',
        properties: props,
      });
      onSuccess('Document created from OCR');
    } catch (e) {
      onError(e, 'Import failed');
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <AppShell title="OCR Scanner" onBack={() => navigate(-1)}>
      <div
        style={{
          maxWidth: 640,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        <Card>
          <div style={{ textAlign: 'center', padding: 24 }}>
            <Scan
              size={48}
              style={{ marginBottom: 12, opacity: 0.3, color: 'var(--text-tertiary)' }}
            />
            <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 4 }}>OCR Scanner</h2>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 16 }}>
              Extract text from images and documents. Supports English, Chinese, Japanese, and
              Korean.
            </p>
            <Button onClick={handleSelectImage} loading={isScanning}>
              <FileText size={14} style={{ marginRight: 6 }} /> Select Image
            </Button>
          </div>
        </Card>

        {result && (
          <Card>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: 12,
              }}
            >
              <h3 style={{ fontSize: 14, fontWeight: 600 }}>Result</h3>
              <Button size="sm" onClick={handleImportAsObject} loading={isImporting}>
                <Upload size={14} style={{ marginRight: 4 }} /> Import as Object
              </Button>
            </div>

            {result.fields.length > 0 && (
              <div style={{ marginBottom: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
                {result.fields.map((f, i) => (
                  <div
                    key={i}
                    style={{
                      display: 'flex',
                      gap: 12,
                      padding: '6px 8px',
                      borderRadius: 6,
                      background: 'var(--bg-toolbar)',
                      fontSize: 13,
                    }}
                  >
                    <span style={{ fontWeight: 500, color: 'var(--text-secondary)', minWidth: 80 }}>
                      {f.label}
                    </span>
                    <span>{f.value}</span>
                    <span
                      style={{ marginLeft: 'auto', fontSize: 11, color: 'var(--text-tertiary)' }}
                    >
                      {(f.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                ))}
              </div>
            )}

            {result.text && (
              <div
                style={{
                  padding: 12,
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  fontSize: 13,
                  lineHeight: 1.6,
                  whiteSpace: 'pre-wrap',
                  maxHeight: 300,
                  overflowY: 'auto',
                }}
              >
                {result.text}
              </div>
            )}

            {!result.text && result.fields.length === 0 && (
              <p
                style={{
                  textAlign: 'center',
                  color: 'var(--text-tertiary)',
                  padding: 24,
                  fontSize: 13,
                }}
              >
                No text detected. Try a clearer image or a different language.
              </p>
            )}
          </Card>
        )}
      </div>
    </AppShell>
  );
}
