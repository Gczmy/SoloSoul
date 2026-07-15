import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X, ZoomIn, ZoomOut, RotateCcw, ExternalLink } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { previewItemByMime } from '@/lib/attachmentUtils';
import { ICON_SIZE } from '@/lib/constants';

interface AttachmentPreviewOverlayProps {
  item: AttachmentItem | null;
  onClose: () => void;
  onOpenExternal?: (item: AttachmentItem) => void;
}

type PreviewKind = 'image' | 'pdf' | 'text' | 'other';

const MIN_SCALE = 0.1;
const MAX_SCALE = 5.0;
const ZOOM_STEP = 1.2;

function isUriPath(path: string): boolean {
  return path.startsWith('content://') || path.startsWith('file://');
}

/**
 * Full-screen attachment preview overlay.
 * Supports image (with zoom), PDF, and text previews.
 * Non-previewable files show an "open externally" fallback.
 *
 * Security:
 * - Only uses `item.vaultPath`; if it is missing or still a content:// URI,
 *   the overlay reports an error instead of leaking the original source URI to Rust.
 */
export function AttachmentPreviewOverlay({
  item,
  onClose,
  onOpenExternal,
}: AttachmentPreviewOverlayProps) {
  const [previewKind, setPreviewKind] = useState<PreviewKind | null>(null);
  const [previewUrl, setPreviewUrl] = useState('');
  const [textContent, setTextContent] = useState('');
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(false);
  const [naturalSize, setNaturalSize] = useState<{ width: number; height: number } | null>(null);
  const [scale, setScale] = useState(1);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!item) {
      setPreviewKind(null);
      setPreviewUrl('');
      setTextContent('');
      setError(false);
      setLoading(false);
      setNaturalSize(null);
      setScale(1);
      return;
    }

    const kind = previewItemByMime(item);
    setPreviewKind(kind);
    setPreviewUrl('');
    setTextContent('');
    setError(false);
    setLoading(true);
    setNaturalSize(null);
    setScale(1);

    const filePath = item.vaultPath;
    if (!filePath || isUriPath(filePath)) {
      setError(true);
      setLoading(false);
      return;
    }

    if (kind === 'image' || kind === 'pdf') {
      invoke<string>('fs_read_file_as_data_url', { path: filePath })
        .then(setPreviewUrl)
        .catch(() => setError(true))
        .finally(() => setLoading(false));
    } else if (kind === 'text') {
      invoke<string>('fs_read_file_as_text', { path: filePath })
        .then(setTextContent)
        .catch(() => setError(true))
        .finally(() => setLoading(false));
    } else {
      // 'other' files are not loaded automatically.
      setLoading(false);
    }
  }, [item]);

  // Calculate an initial scale that fits the image inside the viewport.
  const fitToView = useCallback(() => {
    const container = scrollRef.current;
    if (!container || !naturalSize) return;
    const { clientWidth, clientHeight } = container;
    if (naturalSize.width === 0 || naturalSize.height === 0) return;
    const scaleX = clientWidth / naturalSize.width;
    const scaleY = clientHeight / naturalSize.height;
    const fitScale = Math.min(scaleX, scaleY, 1);
    setScale(Number(fitScale.toFixed(3)));
  }, [naturalSize]);

  useEffect(() => {
    if (naturalSize) {
      fitToView();
    }
  }, [naturalSize, fitToView]);

  useEffect(() => {
    const handleResize = () => {
      if (naturalSize) {
        fitToView();
      }
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [naturalSize, fitToView]);

  const clampScale = (value: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));

  const zoomIn = () => setScale((s) => clampScale(s * ZOOM_STEP));
  const zoomOut = () => setScale((s) => clampScale(s / ZOOM_STEP));
  const resetZoom = () => fitToView();

  const handleWheel = (e: React.WheelEvent) => {
    // Ctrl/Cmd + wheel zooms; plain wheel scrolls the container normally.
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const factor = e.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP;
      setScale((s) => clampScale(s * factor));
    }
  };

  const handleImageLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    setNaturalSize({ width: img.naturalWidth, height: img.naturalHeight });
  };

  if (!item) return null;

  const displayWidth = naturalSize ? Math.round(naturalSize.width * scale) : undefined;
  const displayHeight = naturalSize ? Math.round(naturalSize.height * scale) : undefined;

  const renderContent = () => {
    if (error) {
      return (
        <div style={{ color: '#e74c3c', padding: 24, textAlign: 'center' }}>
          <div>Failed to load preview.</div>
          {(!item.vaultPath || isUriPath(item.vaultPath)) && (
            <div style={{ marginTop: 8, fontSize: 'var(--text-body-sm)' }}>
              Attachment is not stored in vault.
            </div>
          )}
        </div>
      );
    }

    if (loading) {
      return <LoadingPlaceholder variant="toolbar" minHeight={120} />;
    }

    switch (previewKind) {
      case 'image':
        return previewUrl ? (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              display: 'inline-block',
              width: displayWidth,
              height: displayHeight,
              minWidth: 'auto',
              minHeight: 'auto',
              flexShrink: 0,
            }}
          >
            <img
              src={previewUrl}
              alt={item.fileName}
              onLoad={handleImageLoad}
              style={{
                width: '100%',
                height: '100%',
                objectFit: 'contain',
                borderRadius: 8,
                display: 'block',
              }}
            />
          </div>
        ) : null;

      case 'pdf':
        return previewUrl ? (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              width: '100%',
              height: '100%',
              maxWidth: 1024,
              background: '#fff',
              borderRadius: 8,
              overflow: 'hidden',
            }}
          >
            <embed
              src={previewUrl}
              type="application/pdf"
              width="100%"
              height="100%"
              title={item.fileName}
            />
          </div>
        ) : null;

      case 'text':
        return (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              width: '100%',
              maxWidth: 960,
              background: 'var(--bg-elevated)',
              borderRadius: 8,
              padding: 24,
              overflow: 'auto',
            }}
          >
            <pre
              style={{
                margin: 0,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
                fontFamily: 'monospace',
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-primary)',
              }}
            >
              {textContent}
            </pre>
          </div>
        );

      case 'other':
      default:
        return (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              color: 'var(--text-primary)',
              padding: 24,
              textAlign: 'center',
            }}
          >
            <div>Preview is not supported for this file type.</div>
            {onOpenExternal && (
              <button
                onClick={() => onOpenExternal(item)}
                style={{
                  marginTop: 16,
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                  cursor: 'pointer',
                }}
              >
                <ExternalLink size={ICON_SIZE.md} />
                <span>Open with system app</span>
              </button>
            )}
          </div>
        );
    }
  };

  const showZoomControls = previewKind === 'image' && !loading && !error;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-preview-overlay)',
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
        <span style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>{item.fileName}</span>
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
          <X size={ICON_SIZE.lg} />
        </button>
      </div>

      <div
        ref={scrollRef}
        onWheel={handleWheel}
        style={{
          flex: 1,
          overflow: 'auto',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'flex-start',
          padding: 24,
        }}
        onClick={(e) => {
          // Clicks on the empty backdrop still close the overlay.
          if (e.target === e.currentTarget) {
            onClose();
          }
        }}
      >
        {renderContent()}
      </div>

      {/* Zoom controls (image only) */}
      {showZoomControls && (
        <div
          onClick={(e) => e.stopPropagation()}
          style={{
            position: 'absolute',
            bottom: 24,
            left: '50%',
            transform: 'translateX(-50%)',
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            padding: '8px 14px',
            borderRadius: 24,
            background: 'rgba(28,28,30,0.85)',
            border: '1px solid rgba(255,255,255,0.12)',
            color: 'var(--text-primary)',
            fontSize: 'var(--text-body-sm)',
            userSelect: 'none',
          }}
        >
          <button
            onClick={zoomOut}
            title="缩小"
            style={{
              background: 'transparent',
              border: 'none',
              color: 'inherit',
              cursor: 'pointer',
              padding: 4,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <ZoomOut size={ICON_SIZE.lg} />
          </button>
          <span style={{ minWidth: 52, textAlign: 'center' }}>{Math.round(scale * 100)}%</span>
          <button
            onClick={zoomIn}
            title="放大"
            style={{
              background: 'transparent',
              border: 'none',
              color: 'inherit',
              cursor: 'pointer',
              padding: 4,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <ZoomIn size={ICON_SIZE.lg} />
          </button>
          <button
            onClick={resetZoom}
            title="适应窗口"
            style={{
              background: 'transparent',
              border: 'none',
              color: 'inherit',
              cursor: 'pointer',
              padding: 4,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <RotateCcw size={ICON_SIZE.lg} />
          </button>
        </div>
      )}
    </div>
  );
}
