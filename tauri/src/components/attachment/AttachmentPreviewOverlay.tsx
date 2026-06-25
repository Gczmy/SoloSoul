import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import type { AttachmentItem } from '@/lib/attachmentUtils';

interface AttachmentPreviewOverlayProps {
  item: AttachmentItem | null;
  onClose: () => void;
}

/**
 * Full-screen image preview overlay.
 * Reads the file as a data URL via Tauri IPC and displays it.
 * Closes on backdrop click or close button.
 */
export function AttachmentPreviewOverlay({ item, onClose }: AttachmentPreviewOverlayProps) {
  const [previewUrl, setPreviewUrl] = useState('');

  useEffect(() => {
    if (!item) return;
    setPreviewUrl('');
    const filePath = item.vaultPath || item.srcPath;
    if (filePath) {
      invoke<string>('fs_read_file_as_data_url', { path: filePath })
        .then(setPreviewUrl)
        .catch(() => setPreviewUrl('error'));
    } else {
      setPreviewUrl('error');
    }
  }, [item]);

  if (!item) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        display: 'flex',
        flexDirection: 'column',
        background: 'rgba(0,0,0,0.8)',
        backdropFilter: 'blur(12px)',
      }}
      onClick={onClose}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 18px',
          background: 'var(--bg-toolbar)',
        }}
      >
        <span style={{ fontSize: 13, fontWeight: 500 }}>{item.fileName}</span>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onClose();
          }}
          style={{
            color: 'var(--text-secondary)',
            background: 'transparent',
            border: 'none',
            cursor: 'pointer',
          }}
        >
          <X size={18} />
        </button>
      </div>
      <div
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: 24,
        }}
      >
        {previewUrl === 'error' ? (
          <div style={{ color: '#e74c3c', padding: 24 }}>Failed to load preview.</div>
        ) : previewUrl ? (
          <img
            src={previewUrl}
            alt={item.fileName}
            style={{
              maxWidth: '90%',
              maxHeight: '90%',
              objectFit: 'contain',
              borderRadius: 8,
            }}
          />
        ) : (
          <LoadingPlaceholder variant="toolbar" minHeight={120} />
        )}
      </div>
    </div>
  );
}
