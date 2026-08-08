import { useTranslation } from 'react-i18next';
import { Paperclip, RotateCw, Upload, X } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { ICON_SIZE } from '@/lib/constants';

interface AttachmentViewerHeaderProps {
  isNarrowViewport: boolean;
  showTrash: boolean;
  activeCount: number;
  trashCount: number;
  uploading: boolean;
  onShowActive: () => void;
  onShowTrash: () => void;
  onAdd: () => void;
  onClose: () => void;
}

/** AttachmentViewer 顶部区域：标题 + 活跃/回收站切换 + 上传/关闭操作（P013 拆分）。 */
export function AttachmentViewerHeader({
  isNarrowViewport,
  showTrash,
  activeCount,
  trashCount,
  uploading,
  onShowActive,
  onShowTrash,
  onAdd,
  onClose,
}: AttachmentViewerHeaderProps) {
  const { t } = useTranslation(['common', 'editor']);
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '14px 18px',
        borderBottom: '1px solid var(--border-subtle)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <div
          style={{
            fontSize: 'var(--text-body-sm)',
            fontWeight: 600,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          {/* 窄视口只留图标，把空间让给 活跃/回收站 切换与右侧操作按钮 */}
          <Paperclip size={ICON_SIZE.sm} />
          {!isNarrowViewport && t('common:attachments')}
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          <Button
            variant="secondary"
            size="sm"
            onClick={onShowActive}
            style={{
              fontSize: 'var(--text-caption)',
              ...(!showTrash
                ? {
                    background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                    borderColor: 'var(--accent-primary)',
                    color: 'var(--accent-primary)',
                    boxShadow: '0 0 0 1px var(--accent-primary)',
                  }
                : {}),
            }}
          >
            {t('common:attachments_active', { n: activeCount })}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={onShowTrash}
            className="interactive-danger-tab"
            style={{
              fontSize: 'var(--text-caption)',
              ...(showTrash
                ? {
                    background: 'color-mix(in srgb, #e74c3c 10%, transparent)',
                    borderColor: '#e74c3c',
                    color: '#e74c3c',
                    boxShadow: '0 0 0 1px #e74c3c',
                  }
                : {}),
            }}
          >
            {t('common:attachments_trash', { n: trashCount })}
          </Button>
        </div>
      </div>
      {/* 右侧操作：窄视口下 Upload 改纯图标（与关闭按钮同款 44×44），避免头部溢出 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
        {!showTrash &&
          (isNarrowViewport ? (
            <BadgeIconButton
              Icon={Upload}
              onClick={onAdd}
              title={t('common:upload', { defaultValue: 'Upload' })}
              iconSize={ICON_SIZE.sm}
              disabled={uploading}
            />
          ) : (
            <Button variant="secondary" size="sm" onClick={onAdd} disabled={uploading}>
              {uploading ? (
                <RotateCw size={ICON_SIZE.sm} style={{ animation: 'spin 1s linear infinite' }} />
              ) : (
                <Upload size={ICON_SIZE.sm} />
              )}{' '}
              {t('common:upload')}
            </Button>
          ))}
        <BadgeIconButton
          Icon={X}
          onClick={onClose}
          title={t('common:close', { defaultValue: 'Close' })}
          iconSize={ICON_SIZE.md}
        />
      </div>
    </div>
  );
}
