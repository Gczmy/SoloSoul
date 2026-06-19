import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { Scan, History, ArrowUpRight, X } from 'lucide-react';

interface OcrPopoverHeaderProps {
  showHistory: boolean;
  onToggleHistory: () => void;
  onClose: () => void;
}

export function OcrPopoverHeader({ showHistory, onToggleHistory, onClose }: OcrPopoverHeaderProps) {
  const { t } = useTranslation(['ocr', 'common']);
  const navigate = useNavigate();

  const hoverEnter = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
    e.currentTarget.style.color = 'var(--accent-primary)';
  };
  const hoverLeaveDefault = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background = 'transparent';
    e.currentTarget.style.color = 'var(--text-secondary)';
  };

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
        <Scan size={16} style={{ color: 'var(--accent-primary)' }} />
        <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
          {t('ocr:quick_scan_title')}
        </span>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
        <button
          onClick={onToggleHistory}
          title={t('ocr:scan_history')}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
            if (!showHistory) e.currentTarget.style.color = 'var(--accent-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'transparent';
            if (!showHistory) e.currentTarget.style.color = 'var(--text-secondary)';
          }}
          style={{
            padding: 4,
            borderRadius: 6,
            border: 'none',
            background: 'transparent',
            cursor: 'pointer',
            color: showHistory ? 'var(--accent-primary)' : 'var(--text-secondary)',
            transition: 'all 0.15s ease',
          }}
        >
          <History size={14} />
        </button>
        <button
          onClick={() => {
            onClose();
            navigate('/ocr');
          }}
          title={t('ocr:go_to_full_page')}
          onMouseEnter={hoverEnter}
          onMouseLeave={hoverLeaveDefault}
          style={{
            padding: 4,
            borderRadius: 6,
            border: 'none',
            background: 'transparent',
            cursor: 'pointer',
            color: 'var(--text-secondary)',
            transition: 'all 0.15s ease',
          }}
        >
          <ArrowUpRight size={14} />
        </button>
        <button
          onClick={onClose}
          title={t('common:close')}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
            e.currentTarget.style.color = 'var(--accent-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'transparent';
            e.currentTarget.style.color = 'var(--text-tertiary)';
          }}
          style={{
            padding: 4,
            borderRadius: 6,
            border: 'none',
            background: 'transparent',
            cursor: 'pointer',
            color: 'var(--text-tertiary)',
            transition: 'all 0.15s ease',
          }}
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
