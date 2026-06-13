import { useTranslation } from 'react-i18next';
import styles from './PluginResultPanel.module.css';
import type { PluginResultPayload } from '@/lib/plugin';

interface PluginResultPanelProps {
  results: PluginResultPayload[];
}

export function PluginResultPanel({ results }: PluginResultPanelProps) {
  const { t } = useTranslation('plugin');

  if (results.length === 0) {
    return (
      <div className={styles.empty}>
        {t('result_empty', { defaultValue: 'No result yet' })}
      </div>
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

function ResultContent({ payload }: { payload: PluginResultPayload }) {
  const { t } = useTranslation('plugin');

  switch (payload.type) {
    case 'text':
      return <p className={styles.text}>{payload.content}</p>;

    case 'key_value':
      return (
        <>
          {payload.title && <h5 className={styles.title}>{payload.title}</h5>}
          <dl className={styles.keyValue}>
            {payload.pairs.map((pair, idx) => (
              <div key={idx} className={styles.kvRow}>
                <dt>{pair.key}</dt>
                <dd>{pair.value}</dd>
              </div>
            ))}
          </dl>
        </>
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
