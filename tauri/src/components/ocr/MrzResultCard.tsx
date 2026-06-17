import { ShieldCheck, ShieldAlert, ChevronDown, ChevronUp } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { MrzResult } from '@/lib/ipc';

interface MrzResultCardProps {
  result: MrzResult;
}

export function MrzResultCard({ result }: MrzResultCardProps) {
  const { t } = useTranslation('ocr');
  const [showRaw, setShowRaw] = useState(false);

  const fields = [
    { label: t('ocr:mrz_field_type'), value: `${result.documentType} (${result.documentTypeSub})` },
    { label: t('ocr:mrz_field_country'), value: result.issuingCountry },
    { label: t('ocr:mrz_field_number'), value: result.documentNumber },
    { label: t('ocr:mrz_field_nationality'), value: result.nationality },
    { label: t('ocr:mrz_field_dob'), value: result.dateOfBirth },
    { label: t('ocr:mrz_field_sex'), value: result.sex },
    { label: t('ocr:mrz_field_expiry'), value: result.expiryDate },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Checksum status */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '10px 12px',
          borderRadius: 8,
          background: result.checksumValid ? 'var(--accent-primary)' : 'var(--error)',
          color: '#fff',
          fontSize: 13,
          fontWeight: 500,
        }}
      >
        {result.checksumValid ? <ShieldCheck size={16} /> : <ShieldAlert size={16} />}
        {result.checksumValid ? t('ocr:mrz_checksum_valid') : t('ocr:mrz_checksum_invalid')}
      </div>

      {/* Field grid */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
          gap: 10,
        }}
      >
        {fields.map((field) => (
          <div
            key={field.label}
            style={{
              padding: '8px 10px',
              borderRadius: 6,
              background: 'var(--bg-toolbar)',
            }}
          >
            <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 2 }}>
              {field.label}
            </div>
            <div style={{ fontSize: 13, fontWeight: 500, wordBreak: 'break-word' }}>
              {field.value || '-'}
            </div>
          </div>
        ))}
      </div>

      {/* Raw lines toggle */}
      <button
        onClick={() => setShowRaw(!showRaw)}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          padding: '6px 0',
          fontSize: 12,
          color: 'var(--text-secondary)',
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          alignSelf: 'flex-start',
        }}
      >
        {showRaw ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        {t('ocr:mrz_raw_lines')}
      </button>

      {showRaw && (
        <div
          style={{
            padding: 10,
            borderRadius: 6,
            background: 'var(--bg-toolbar)',
            fontFamily: 'monospace',
            fontSize: 12,
            lineHeight: 1.5,
            color: 'var(--text-secondary)',
          }}
        >
          {result.rawLines.map((line, i) => (
            <div key={i}>{line}</div>
          ))}
        </div>
      )}
    </div>
  );
}
