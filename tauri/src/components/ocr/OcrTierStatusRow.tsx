import { CheckCircle, AlertCircle, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { ICON_SIZE } from '@/lib/constants';
import type { ReactNode } from 'react';

export interface OcrTierStatus {
  installed?: boolean;
  bundled?: boolean;
  builtin?: boolean;
}

interface OcrTierStatusRowProps {
  tierKey: string;
  status: OcrTierStatus | undefined;
  /** tier 显示名（调用方已本地化）。 */
  label: string;
  /** 状态文本（调用方已本地化，含存储占用等差异）。 */
  statusText: string;
  isInstalling?: boolean;
  isDownloading?: boolean;
  isDeleting?: boolean;
  onInstall?: () => void;
  onDownload?: () => void;
  onDelete?: () => void;
  /** 可选删除按钮内容（含图标；默认 common:delete 纯文本）。 */
  deleteLabel?: ReactNode;
  /** 是否可用裸按钮样式（interactive-toolbar，默认 false 用 Button 组件）。 */
  rawButton?: boolean;
}

/**
 * OCR tier 状态行（P041：OcrScanSettingsPanel 与 OcrSettingsPage 的状态行布局一致，
 * 收敛为共享组件；差异通过 props 参数化）。
 */
export function OcrTierStatusRow({
  tierKey,
  status,
  label,
  statusText,
  isInstalling = false,
  isDownloading = false,
  isDeleting = false,
  onInstall,
  onDownload,
  onDelete,
  deleteLabel,
  rawButton = false,
}: OcrTierStatusRowProps) {
  const { t } = useTranslation(['ocr', 'common']);

  const renderButton = (content: ReactNode, className: string, onClick?: () => void, disabled = false, loading = false) =>
    rawButton ? (
      <button
        onClick={onClick}
        disabled={disabled}
        className={className}
        style={{
          padding: '6px 12px',
          borderRadius: 8,
          borderWidth: 1,
          borderStyle: 'solid',
          fontSize: 'var(--text-caption)',
          fontWeight: 500,
          cursor: disabled ? 'default' : 'pointer',
          opacity: disabled ? 0.6 : 1,
          fontFamily: 'inherit',
          display: 'inline-flex',
          alignItems: 'center',
          gap: 4,
          whiteSpace: 'nowrap',
        }}
      >
        {loading ? t('common:loading', { defaultValue: '...' }) : content}
      </button>
    ) : (
      <Button size="sm" onClick={onClick} loading={loading} disabled={disabled}>
        {content}
      </Button>
    );

  return (
    <div
      key={tierKey}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 12px',
        borderRadius: 8,
        background: 'var(--bg-toolbar)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
        {status?.installed ? (
          <CheckCircle size={ICON_SIZE.md} color="var(--accent-primary)" />
        ) : status?.bundled ? (
          <AlertCircle size={ICON_SIZE.md} color="var(--text-tertiary)" />
        ) : (
          <AlertCircle size={ICON_SIZE.md} color="var(--error)" />
        )}
        <div>
          <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>{label}</div>
          <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
            {statusText}
          </div>
        </div>
      </div>
      <div style={{ display: 'flex', gap: 8 }}>
        {status?.bundled && !status?.installed && onInstall && (
          <>{renderButton(t('ocr:install'), 'interactive-toolbar', onInstall, isInstalling, isInstalling)}</>
        )}
        {!status?.bundled && !status?.installed && onDownload && (
          <>
            {renderButton(
              <>
                <Download size={ICON_SIZE.sm} />
                {t('ocr:download')}
              </>,
              'interactive-toolbar',
              onDownload,
              isDownloading,
              isDownloading,
            )}
          </>
        )}
        {status?.installed && !status?.builtin && onDelete && (
          <>{renderButton(deleteLabel ?? t('common:delete'), 'interactive-danger', onDelete, isDeleting, isDeleting)}</>
        )}
      </div>
    </div>
  );
}
