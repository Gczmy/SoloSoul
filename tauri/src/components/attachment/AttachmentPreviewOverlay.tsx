import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowLeft, X, ZoomIn, ZoomOut, RotateCcw, ExternalLink, FilePen } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { AttachmentMetaEditDialog } from '@/components/attachment/AttachmentMetaEditDialog';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { ICON_SIZE, SAFE_AREA_TOP, SAFE_AREA_BOTTOM } from '@/lib/constants';
// P048: 加载/缩放/手势逻辑抽到 hook
import { useAttachmentPreview, isUriPath } from './useAttachmentPreview';

interface AttachmentPreviewOverlayProps {
  item: AttachmentItem | null;
  onClose: () => void;
  onOpenExternal?: (item: AttachmentItem) => void;
  /** 附件描述/标签保存成功后的回调（父级同步列表状态） */
  onItemUpdated?: (item: AttachmentItem) => void;
  /**
   * 只读上下文（如回收站详情）：隐藏「编辑附件属性」按钮（对象已删，改名/改
   * 描述无意义且不落库）；vaultPath 缺失时提示「文件已不存在」而非「未存储在
   * 保险库」（回收站里附件文件可能已随附件级永久删除而消失）。
   */
  disableMetaEdit?: boolean;
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
  onItemUpdated,
  disableMetaEdit = false,
}: AttachmentPreviewOverlayProps) {
  const { t } = useTranslation('common');
  const [metaEditOpen, setMetaEditOpen] = useState(false);
  const {
    previewKind,
    previewUrl,
    textContent,
    error,
    loading,
    naturalSize,
    scale,
    scrollRef,
    fitsViewport,
    zoomIn,
    zoomOut,
    resetZoom,
    handleWheel,
    handleImageLoad,
  } = useAttachmentPreview({ item });

  if (!item) return null;

  const displayWidth = naturalSize ? Math.round(naturalSize.width * scale) : undefined;
  const displayHeight = naturalSize ? Math.round(naturalSize.height * scale) : undefined;

  const renderContent = () => {
    if (error) {
      return (
        <div style={{ margin: 'auto', color: '#e74c3c', padding: 24, textAlign: 'center' }}>
          <div>{t('common:attachment_preview_failed', 'Failed to load preview.')}</div>
          {(!item.vaultPath || isUriPath(item.vaultPath)) && (
            <div style={{ marginTop: 8, fontSize: 'var(--text-body-sm)' }}>
              {/* 回收站只读上下文：文件已随附件级永久删除消失；常规上下文：从未入库 */}
              {disableMetaEdit
                ? t('common:attachment_file_missing', 'The attachment file no longer exists.')
                : t('common:attachment_not_in_vault', 'Attachment is not stored in vault.')}
            </div>
          )}
        </div>
      );
    }

    if (loading) {
      return <LoadingPlaceholder variant="toolbar" minHeight={120} style={{ margin: 'auto' }} />;
    }

    switch (previewKind) {
      case 'image':
        return previewUrl ? (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              // margin: auto——内容小于视口时水平垂直居中；放大超出视口时
              // auto 边距归零、按起始位置对齐，保证溢出边缘仍可滚动到达（与
              // PhotoViewerOverlay 的居中策略一致，避免 flex 居中裁剪顶部）。
              margin: 'auto',
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
              // margin 0 auto：宽视口（> 1024px）下保持水平居中（容器已移除
              // justifyContent: center，改为内容自身 margin 居中）
              margin: '0 auto',
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
              margin: 'auto',
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
              margin: 'auto',
              color: 'var(--text-primary)',
              padding: 24,
              textAlign: 'center',
            }}
          >
            <div>
              {t(
                'common:attachment_preview_unsupported',
                'Preview is not supported for this file type.',
              )}
            </div>
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
                <span>{t('common:attachment_open_system', 'Open with system app')}</span>
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
        top: SAFE_AREA_TOP,
        left: 0,
        right: 0,
        bottom: SAFE_AREA_BOTTOM,
        zIndex: 'var(--z-preview-overlay)',
        display: 'flex',
        flexDirection: 'column',
        background: 'rgba(0,0,0,0.8)',
        backdropFilter: 'blur(12px)',
      }}
      onClick={(e) => {
        // 阻止冒泡：预览遮罩是 AttachmentViewer/附件管理器容器的子层，
        // 点击预览背景仅关闭预览本身，不能让事件继续冒泡把整个附件卡片关掉
        // （否则会直接跳回 workspace/详情卡片）。
        e.stopPropagation();
        onClose();
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '10px 14px',
          background: 'var(--bg-toolbar)',
        }}
      >
        {/* 左上角返回按钮：与照片集全屏查看器一致，返回附件列表 */}
        <button
          onClick={(e) => {
            e.stopPropagation();
            onClose();
          }}
          title={t('common:back', 'Back')}
          aria-label={t('common:back', 'Back')}
          className="interactive-icon"
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: 36,
            height: 36,
            borderRadius: 8,
            border: 'none',
            background: 'transparent',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
            flexShrink: 0,
          }}
        >
          <ArrowLeft size={ICON_SIZE.md} />
        </button>
        <span
          style={{
            flex: 1,
            minWidth: 0,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
          }}
        >
          {item.fileName}
        </span>
        {/* 描述/标签编辑入口（全屏照片，需求 2+3）。回收站只读上下文隐藏（disableMetaEdit） */}
        {!disableMetaEdit && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setMetaEditOpen(true);
            }}
            title={t('common:edit_meta', 'Edit Attachment Attributes')}
            aria-label={t('common:edit_meta', 'Edit Attachment Attributes')}
            className="interactive-icon"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: 36,
              height: 36,
              borderRadius: 8,
              border: 'none',
              background: 'transparent',
              color: 'var(--text-secondary)',
              cursor: 'pointer',
              flexShrink: 0,
            }}
          >
            <FilePen size={ICON_SIZE.md} />
          </button>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation();
            onClose();
          }}
          title={t('common:close', 'Close')}
          aria-label={t('common:close', 'Close')}
          className="interactive-icon"
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: 36,
            height: 36,
            borderRadius: 8,
            border: 'none',
            background: 'transparent',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
            flexShrink: 0,
          }}
        >
          <X size={ICON_SIZE.lg} />
        </button>
      </div>

      {/* 描述/标签编辑对话框 */}
      {metaEditOpen && (
        <AttachmentMetaEditDialog
          item={item}
          onClose={() => setMetaEditOpen(false)}
          onSaved={(updated) => {
            onItemUpdated?.({ ...item, ...updated });
            setMetaEditOpen(false);
          }}
        />
      )}

      {/* 背景点击统一由外层 div 处理（stopPropagation + onClose）：
          此处不再挂 onClick，避免与外层重复触发 onClose 两次。
          内容包装器已各自 stopPropagation，点击图片/PDF/文本不会关闭。 */}
      <div
        ref={scrollRef}
        data-testid="attachment-preview-content"
        onWheel={handleWheel}
        style={{
          flex: 1,
          overflow: 'auto',
          display: 'flex',
          padding: 24,
          // 仅图片预览覆写：未超出视口时声明本区域仅纵向 pan（把双指/双击手势
          // 让给 JS 的 useTouchZoom，浏览器不抢）；放大超出后恢复 auto 双向滚动平移
          touchAction: previewKind === 'image' ? (fitsViewport ? 'pan-y' : 'auto') : undefined,
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
            title={t('common:attachment_zoom_out', 'Zoom Out')}
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
            title={t('common:attachment_zoom_in', 'Zoom In')}
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
            title={t('common:attachment_zoom_fit', 'Fit to window')}
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
