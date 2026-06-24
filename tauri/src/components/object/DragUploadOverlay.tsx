import { useTranslation } from 'react-i18next';
import { Upload } from 'lucide-react';
import type { DragUploadState } from '@/hooks/useDragToAttach';

interface DragUploadOverlayProps {
  dragState: DragUploadState;
  /**
   * 覆盖层的圆角值，与被包裹元素的 border-radius 保持一致。
   * @default 12
   */
  borderRadius?: number;
}

const pulseKeyframes = `
@keyframes dragPulse {
  0%, 100% { opacity: 0.6; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.1); }
}`;

/**
 * DragUploadOverlay
 *
 * 半透明覆盖层，在拖拽文件到目标区域或正在上传文件时显示提示。
 * - 拖拽悬停时：显示「释放以上传文件」
 * - 上传中：显示上传进度（如 "上传中 (2/5): 文件名.pdf"）
 * - 空闲时：不渲染任何内容
 */
export function DragUploadOverlay({
  dragState,
  borderRadius = 12,
}: DragUploadOverlayProps) {
  const { t } = useTranslation('common');
  const { isDraggingOver, isUploading, currentIndex, totalFiles, currentFileName, pendingFiles } = dragState;

  if (!isDraggingOver && !isUploading) return null;

  const overlayStyle: React.CSSProperties = {
    position: 'absolute',
    inset: 0,
    borderRadius,
    zIndex: 100,
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
    pointerEvents: 'none',
    transition: 'opacity 0.15s ease',
  };

  // 上传中状态
  if (isUploading) {
    return (
      <>
        <style>{pulseKeyframes}</style>
        <div
          style={{
            ...overlayStyle,
            background: 'rgba(0, 0, 0, 0.55)',
            backdropFilter: 'blur(6px)',
          }}
        >
          <Upload
            size={28}
            style={{ color: 'var(--accent-primary)', animation: 'dragPulse 1.2s ease-in-out infinite' }}
          />
          <div style={{ color: '#fff', fontSize: 13, fontWeight: 600, textAlign: 'center' }}>
            {totalFiles > 1
              ? t('uploads_in_progress', { current: currentIndex + 1, total: totalFiles })
              : t('uploading')}
          </div>
          <div
            style={{
              color: 'rgba(255,255,255,0.7)',
              fontSize: 11,
              maxWidth: 240,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {currentFileName}
          </div>
          {/* 队列等待提示 */}
          {pendingFiles > 0 && (
            <div
              style={{
                color: 'rgba(255,255,255,0.5)',
                fontSize: 10,
                marginTop: 2,
              }}
            >
              {t('uploads_queued', { n: pendingFiles })}
            </div>
          )}
          {/* 进度条 */}
          <div
            style={{
              width: 120,
              height: 3,
              borderRadius: 2,
              background: 'rgba(255,255,255,0.2)',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                height: '100%',
                width: `${totalFiles > 1 ? ((currentIndex + 1) / totalFiles) * 100 : 30}%`,
                background: 'var(--accent-primary)',
                borderRadius: 2,
                transition: 'width 0.3s ease',
              }}
            />
          </div>
        </div>
      </>
    );
  }

  // 拖拽悬停状态
  if (isDraggingOver) {
    return (
      <div
        style={{
          ...overlayStyle,
          background: 'color-mix(in srgb, var(--accent-primary) 12%, transparent)',
          border: '2px dashed var(--accent-primary)',
        }}
      >
        <Upload size={24} style={{ color: 'var(--accent-primary)' }} />
        <span style={{ color: 'var(--accent-primary)', fontSize: 13, fontWeight: 600 }}>
          {t('drop_to_upload')}
        </span>
        <span style={{ color: 'var(--text-tertiary)', fontSize: 11 }}>
          {t('drop_multiple_hint')}
        </span>
      </div>
    );
  }

  return null;
}
