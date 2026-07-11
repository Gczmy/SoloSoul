import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';

interface RiskAcceptanceDialogProps {
  open: boolean;
  onClose: () => void;
  onAccept: () => void;
}

export function RiskAcceptanceDialog({ open, onClose, onAccept }: RiskAcceptanceDialogProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [riskChecked, setRiskChecked] = useState(false);

  if (!open) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(6px)',
      }}
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: '28px 32px',
          maxWidth: 400,
          width: '90%',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <h3
          style={{
            fontSize: 'var(--text-md)',
            fontWeight: 600,
            marginBottom: 12,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <span style={{ fontSize: 'var(--text-page-title)' }}>⚠</span>{' '}
          {t('settings:ai_risk_title')}
        </h3>
        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            lineHeight: 1.6,
            marginBottom: 16,
          }}
        >
          {t('settings:ai_risk_desc')}
        </p>
        <ul
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
            lineHeight: 1.8,
            paddingLeft: 16,
            marginBottom: 16,
          }}
        >
          <li>{t('settings:ai_risk_li1')}</li>
          <li>{t('settings:ai_risk_li2')}</li>
          <li>{t('settings:ai_risk_li3')}</li>
          <li>{t('settings:ai_risk_li4')}</li>
        </ul>
        <label
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            cursor: 'pointer',
            marginBottom: 16,
            fontSize: 'var(--text-body-sm)',
          }}
        >
          <SelectCheckbox checked={riskChecked} onChange={(v) => setRiskChecked(v)} />
          {t('settings:ai_risk_agree')}
        </label>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onClose}>
            {t('common:cancel')}
          </Button>
          <Button onClick={onAccept} disabled={!riskChecked}>
            {t('settings:ai_enable')}
          </Button>
        </div>
      </div>
    </div>
  );
}
