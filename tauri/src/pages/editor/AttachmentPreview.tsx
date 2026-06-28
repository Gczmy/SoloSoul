import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';


interface AttachmentPreviewProps {
  fileName: string;
  mimeType: string;
  filePath?: string;
  onClose: () => void;
}

export function AttachmentPreview({
  fileName,
  mimeType,
  filePath,
  onClose,
}: AttachmentPreviewProps) {
  const [zoom, setZoom] = useState(100);
  const [previewUrl, setPreviewUrl] = useState('');
  const isImage = mimeType.startsWith('image/');
  const isPDF = mimeType === 'application/pdf';

  useEffect(() => {
    if (!filePath) return;
    invoke<string>('fs_read_file_as_data_url', { path: filePath })
      .then(setPreviewUrl)
      .catch(() => setPreviewUrl('error'));
  }, [filePath]);

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 2000,
        display: 'flex',
        flexDirection: 'column',
        background: 'rgba(0,0,0,0.6)',
        backdropFilter: 'blur(8px)',
      }}
    >
      {/* Toolbar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '8px 16px',
          background: 'var(--bg-toolbar)',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <span style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>{fileName}</span>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          {isImage && (
            <>
              <button onClick={() => setZoom((z) => Math.max(25, z - 25))} style={btnStyle}>
                -
              </button>
              <span style={{ fontSize: 'var(--text-caption)' }}>{zoom}%</span>
              <button onClick={() => setZoom((z) => Math.min(400, z + 25))} style={btnStyle}>
                +
              </button>
            </>
          )}
          <button onClick={onClose} style={{ ...btnStyle, fontSize: 'var(--text-section-title)' }}>
            <X size={ICON_SIZE.lg} />
          </button>
        </div>
      </div>

      {/* Content */}
      <div
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: 16,
        }}
      >
        {previewUrl && previewUrl !== 'error' && (
          <img
            src={previewUrl}
            alt={fileName}
            style={{
              maxWidth: `${zoom}%`,
              maxHeight: `${zoom}%`,
              objectFit: 'contain',
              borderRadius: 8,
            }}
          />
        )}
        {previewUrl === 'error' && (
          <div style={{ color: '#e74c3c', padding: 24 }}>Failed to load preview.</div>
        )}
      </div>
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  width: 32,
  height: 32,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  border: 'none',
  borderRadius: 6,
  background: 'transparent',
  cursor: 'pointer',
  color: 'var(--text-secondary)',
  fontSize: 'var(--text-body)',
  fontWeight: 600,
};
