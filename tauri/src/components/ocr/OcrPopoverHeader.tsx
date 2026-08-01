import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { Scan, History, ArrowUpRight, X } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

interface OcrPopoverHeaderProps {
  showHistory: boolean;
  onToggleHistory: () => void;
  onClose: () => void;
}

export function OcrPopoverHeader({ showHistory, onToggleHistory, onClose }: OcrPopoverHeaderProps) {
  const { t } = useTranslation(['ocr', 'common']);
  const navigate = useNavigate();

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 12px',
        borderBottom: '1px solid var(--border-subtle)',
        flexShrink: 0,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <Scan size={ICON_SIZE.md} style={{ color: 'var(--accent-primary)' }} />
        <span
          style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600, color: 'var(--text-primary)' }}
        >
          {t('ocr:quick_scan_title')}
        </span>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
        <button
          onClick={onToggleHistory}
          title={t('ocr:scan_history')}
          className="interactive-icon"
          style={{
            padding: 4,
            borderRadius: 6,
            border: 'none',
            cursor: 'pointer',
            color: showHistory ? 'var(--accent-primary)' : undefined,
          }}
        >
          <History size={ICON_SIZE.sm} />
        </button>
        <button
          onClick={() => {
            onClose();
            navigate('/ocr');
          }}
          title={t('ocr:go_to_full_page')}
          className="interactive-icon"
          style={{
            padding: 4,
            borderRadius: 6,
            border: 'none',
            cursor: 'pointer',
          }}
        >
          <ArrowUpRight size={ICON_SIZE.sm} />
        </button>
        <button
          onClick={onClose}
          title={t('common:close')}
          className="interactive-accent"
          style={{
            padding: 4,
            borderRadius: 6,
            border: 'none',
            cursor: 'pointer',
          }}
        >
          <X size={ICON_SIZE.sm} />
        </button>
      </div>
    </div>
  );
}
