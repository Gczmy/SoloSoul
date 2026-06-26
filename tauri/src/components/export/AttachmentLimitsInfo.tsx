import { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { Info } from 'lucide-react';

export function AttachmentLimitsInfo() {
  const { t } = useTranslation('settings');
  const [show, setShow] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);

  useEffect(() => {
    if (show && btnRef.current) {
      const rect = btnRef.current.getBoundingClientRect();
      setPos({ top: rect.bottom + 6, left: rect.left });
    }
  }, [show]);

  return (
    <div style={{ display: 'inline-flex', alignItems: 'center' }}>
      <button
        ref={btnRef}
        type="button"
        onMouseEnter={() => setShow(true)}
        onMouseLeave={() => setShow(false)}
        aria-label={t('attachment_limits_title')}
        style={{
          background: 'none',
          border: 'none',
          padding: 2,
          display: 'flex',
          alignItems: 'center',
          color: 'var(--text-tertiary)',
          cursor: 'pointer',
        }}
      >
        <Info size={14} />
      </button>
      {show &&
        pos &&
        createPortal(
          <div
            style={{
              position: 'fixed',
              top: pos.top,
              left: pos.left,
              zIndex: 5000,
              background: 'var(--bg-elevated)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 8,
              padding: 12,
              boxShadow: 'var(--shadow-md)',
              fontSize: 'var(--text-caption)',
              color: 'var(--text-secondary)',
              maxWidth: 520,
              lineHeight: 1.5,
            }}
          >
            <div
              style={{
                fontWeight: 600,
                marginBottom: 8,
                color: 'var(--text-primary)',
              }}
            >
              {t('attachment_limits_title')}
            </div>
            <table style={{ borderCollapse: 'collapse', width: '100%' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                  <th style={{ textAlign: 'left', padding: '4px 8px', fontWeight: 600 }}>
                    {t('attachment_limits_type')}
                  </th>
                  <th style={{ textAlign: 'left', padding: '4px 8px', fontWeight: 600 }}>
                    {t('attachment_limits_threshold')}
                  </th>
                  <th style={{ textAlign: 'left', padding: '4px 8px', fontWeight: 600 }}>
                    {t('attachment_limits_behavior')}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                  <td style={{ padding: '4px 8px' }}>{t('attachment_limit_single_size')}</td>
                  <td style={{ padding: '4px 8px' }}>100 MB</td>
                  <td style={{ padding: '4px 8px' }}>
                    {t('attachment_limit_single_size_behavior')}
                  </td>
                </tr>
                <tr style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                  <td style={{ padding: '4px 8px' }}>{t('attachment_limit_single_count')}</td>
                  <td style={{ padding: '4px 8px' }}>200</td>
                  <td style={{ padding: '4px 8px' }}>
                    {t('attachment_limit_single_count_behavior')}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '4px 8px' }}>{t('attachment_limit_total_size')}</td>
                  <td style={{ padding: '4px 8px' }}>1 GB</td>
                  <td style={{ padding: '4px 8px' }}>
                    {t('attachment_limit_total_size_behavior')}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>,
          document.body,
        )}
    </div>
  );
}
