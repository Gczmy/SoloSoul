import { useTranslation } from 'react-i18next';
import { FileJson, FileText } from 'lucide-react';
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
          <ResultToolbar payload={result} />
          <ResultContent payload={result} />
        </div>
      ))}
    </div>
  );
}

function ResultToolbar({ payload }: { payload: PluginResultPayload }) {
  const { t } = useTranslation('plugin');

  const copyJson = async () => {
    try {
      await navigator.clipboard.writeText(payloadToJson(payload));
    } catch {
      // 剪贴板写入失败时静默忽略，避免打断用户。
    }
  };

  const copyMarkdown = async () => {
    try {
      await navigator.clipboard.writeText(payloadToMarkdown(payload));
    } catch {
      // 同上。
    }
  };

  return (
    <div className={styles.toolbar}>
      <button
        type="button"
        className={styles.toolbarButton}
        onClick={copyJson}
        title={t('copy_json', { defaultValue: 'Copy as JSON' })}
        aria-label={t('copy_json', { defaultValue: 'Copy as JSON' })}
      >
        <FileJson size={14} />
        <span>{t('copy_json_short', { defaultValue: 'JSON' })}</span>
      </button>
      <button
        type="button"
        className={styles.toolbarButton}
        onClick={copyMarkdown}
        title={t('copy_markdown', { defaultValue: 'Copy as Markdown' })}
        aria-label={t('copy_markdown', { defaultValue: 'Copy as Markdown' })}
      >
        <FileText size={14} />
        <span>{t('copy_markdown_short', { defaultValue: 'Markdown' })}</span>
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

function payloadToJson(payload: PluginResultPayload): string {
  return JSON.stringify(payload, null, 2);
}

function escapeMarkdownCell(value: string): string {
  // 将表格单元格中的管道符替换为全角字符，避免破坏 Markdown 表格结构。
  return value.replace(/\|/g, '｜').replace(/\n/g, ' ');
}

function payloadToMarkdown(payload: PluginResultPayload): string {
  switch (payload.type) {
    case 'text':
      return payload.content;

    case 'key_value': {
      const header = payload.title ? `### ${payload.title}\n\n` : '';
      const rows = payload.pairs
        .map((pair) => `| ${escapeMarkdownCell(pair.key)} | ${escapeMarkdownCell(pair.value)} |`)
        .join('\n');
      return `${header}| Key | Value |\n| --- | --- |\n${rows}`;
    }

    case 'table': {
      const header = `| ${payload.headers.map(escapeMarkdownCell).join(' | ')} |`;
      const divider = `| ${payload.headers.map(() => '---').join(' | ')} |`;
      const rows = payload.rows
        .map((row) => `| ${row.map(escapeMarkdownCell).join(' | ')} |`)
        .join('\n');
      return `${header}\n${divider}\n${rows}`;
    }

    case 'markdown':
      return payload.content;

    default:
      return '```json\n' + JSON.stringify(payload, null, 2) + '\n```';
  }
}
