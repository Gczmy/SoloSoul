import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Copy, Check } from 'lucide-react';
import styles from './PluginResultPanel.module.css';
import type { PluginResultPayload } from '@/lib/plugin';

interface PluginResultPanelProps {
  results: PluginResultPayload[];
}

export function PluginResultPanel({ results }: PluginResultPanelProps) {
  const { t } = useTranslation('plugin');

  if (results.length === 0) {
    return (
      <div className={styles.empty}>{t('result_empty', { defaultValue: 'No result yet' })}</div>
    );
  }

  return (
    <div className={styles.container}>
      {results.map((result, index) => (
        <div key={index} className={styles.resultCard}>
          <ResultContent payload={result} />
        </div>
      ))}
    </div>
  );
}

function PerPairCopyRow({
  pair,
}: {
  pair: { key: string; value: string; tag?: string; tagCode?: string };
}) {
  const { t, i18n } = useTranslation('plugin');
  const [copied, setCopied] = useState(false);
  const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';

  const copyPair = async () => {
    try {
      await navigator.clipboard.writeText(`${pair.key}: ${pair.value}`);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // 静默忽略
    }
  };

  const badgeLabel = pair.tag || pair.tagCode;

  return (
    <div className={styles.pairRow}>
      {badgeLabel && (
        <span className={styles.countryBadge} title={pair.tag || pair.tagCode}>
          {locale === 'zh' ? pair.tag || pair.tagCode : pair.tagCode || pair.tag}
        </span>
      )}
      <span className={styles.pairKey}>{pair.key}</span>
      <span className={styles.pairValue}>{pair.value}</span>
      <button
        type="button"
        className={`${styles.pairCopyBtn} ${copied ? styles.pairCopyBtnActive : ''}`}
        onClick={copyPair}
        title={t('copy_entry', { defaultValue: 'Copy this entry' })}
        aria-label={t('copy_entry', { defaultValue: 'Copy this entry' })}
      >
        {copied ? <Check size={12} /> : <Copy size={12} />}
      </button>
    </div>
  );
}

function ResultContent({ payload }: { payload: PluginResultPayload }) {
  const { t } = useTranslation('plugin');

  switch (payload.type) {
    case 'text':
      return <p className={styles.text}>{payload.content}</p>;

    case 'key_value':
      return (
        <div className={styles.keyValueList}>
          {payload.title && <div className={styles.keyValueTitle}>{payload.title}</div>}
          {payload.pairs.map((pair, idx) => (
            <PerPairCopyRow key={idx} pair={pair} />
          ))}
        </div>
      );

    case 'table':
      return (
        <div className={styles.tableWrapper}>
          <table className={styles.table}>
            <thead>
              <tr>
                {payload.headers.map((h, i) => (
                  <th key={i}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {payload.rows.map((row, i) => (
                <tr key={i}>
                  {row.map((cell, j) => (
                    <td key={j}>{cell}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );

    case 'markdown':
      return <pre className={styles.markdown}>{payload.content}</pre>;

    default:
      return (
        <div className={styles.unknown}>
          {t('result_unknown', { defaultValue: 'Unsupported result type' })}
        </div>
      );
  }
}


