import { useState } from 'react';
import { X } from 'lucide-react';

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
  const isImage = mimeType.startsWith('image/');
  const isPDF = mimeType === 'application/pdf';

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
        <span style={{ fontSize: 13, fontWeight: 500 }}>{fileName}</span>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          {isImage && (
            <>
              <button onClick={() => setZoom((z) => Math.max(25, z - 25))} style={btnStyle}>
                -
              </button>
              <span style={{ fontSize: 12 }}>{zoom}%</span>
              <button onClick={() => setZoom((z) => Math.min(400, z + 25))} style={btnStyle}>
                +
              </button>
            </>
          )}
          <button onClick={onClose} style={{ ...btnStyle, fontSize: 16 }}>
            <X size={18} />
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
        {isImage && filePath && (
          <img
            src={`asset://localhost/${encodeURIComponent(filePath)}`}
            alt={fileName}
            style={{
              maxWidth: `${zoom}%`,
              maxHeight: `${zoom}%`,
              objectFit: 'contain',
              borderRadius: 8,
            }}
          />
        )}
        {isPDF && filePath && (
          <iframe
            src={`asset://localhost/${encodeURIComponent(filePath)}`}
            style={{
              width: '100%',
              height: '100%',
              border: 'none',
              borderRadius: 8,
              background: 'white',
            }}
          />
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
  fontSize: 14,
  fontWeight: 600,
};
