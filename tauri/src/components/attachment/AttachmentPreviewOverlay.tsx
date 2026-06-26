import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X, ZoomIn, ZoomOut, RotateCcw } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import type { AttachmentItem } from '@/lib/attachmentUtils';

interface AttachmentPreviewOverlayProps {
  item: AttachmentItem | null;
  onClose: () => void;
}

const MIN_SCALE = 0.1;
const MAX_SCALE = 5.0;
const ZOOM_STEP = 1.2;

/**
 * Full-screen image preview overlay.
 * Reads the file as a data URL via Tauri IPC and displays it.
 * Supports vertical/horizontal scrolling, zoom in/out, and wheel zoom (Ctrl/Cmd + wheel).
 * Closes on backdrop click or close button.
 */
export function AttachmentPreviewOverlay({ item, onClose }: AttachmentPreviewOverlayProps) {
  const [previewUrl, setPreviewUrl] = useState('');
  const [naturalSize, setNaturalSize] = useState<{ width: number; height: number } | null>(null);
  const [scale, setScale] = useState(1);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!item) return;
    setPreviewUrl('');
    setNaturalSize(null);
    setScale(1);
    const filePath = item.vaultPath || item.srcPath;
    if (filePath) {
      invoke<string>('fs_read_file_as_data_url', { path: filePath })
        .then(setPreviewUrl)
        .catch(() => setPreviewUrl('error'));
    } else {
      setPreviewUrl('error');
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

  // Re-fit when the window is resized.
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
        {previewUrl === 'error' ? (
          <div style={{ color: '#e74c3c', padding: 24 }}>Failed to load preview.</div>
        ) : previewUrl ? (
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
        ) : (
          <LoadingPlaceholder variant="toolbar" minHeight={120} />
        )}
      </div>

      {/* Zoom controls */}
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
          fontSize: 13,
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
          <ZoomOut size={18} />
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
          <ZoomIn size={18} />
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
          <RotateCcw size={18} />
        </button>
      </div>
    </div>
  );
}
