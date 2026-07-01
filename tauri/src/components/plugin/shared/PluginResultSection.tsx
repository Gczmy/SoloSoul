import { useTranslation } from 'react-i18next';
import { FileJson, FileText } from 'lucide-react';
import { ExpandableSection } from './ExpandableSection';
import { CopyButton } from './CopyButton';
import { PluginResultPanel } from '../PluginResultPanel';
import type { PluginResultPayload } from '@/lib/plugin';

interface PluginResultSectionProps {
  results: PluginResultPayload[];
  defaultExpanded?: boolean;
  /** Show JSON + Markdown copy buttons (page variant only) */
  showCopyButtons?: boolean;
  /** Size variant */
  variant?: 'page' | 'sidebar';
}

export function PluginResultSection({
  results,
  defaultExpanded = true,
  showCopyButtons = false,
  variant = 'sidebar',
}: PluginResultSectionProps) {
  const { t } = useTranslation('plugin');
  const size = variant === 'page' ? 'md' : 'sm';

  const getJsonContent = () => JSON.stringify(results, null, 2);
  const getMarkdownContent = () =>
    results
      .map((r) => {
        if (r.type === 'key_value') {
          const rows = r.pairs.map((p) => `| ${p.key} | ${p.value} |`).join('\n');
          const header = r.title ? `### ${r.title}\n\n` : '';
          return `${header}| Key | Value |\n| --- | --- |\n${rows}`;
        }
        return JSON.stringify(r, null, 2);
      })
      .join('\n\n---\n\n');

  return (
    <ExpandableSection
      title={t('inline_result', { defaultValue: 'Plugin Result' })}
      defaultExpanded={defaultExpanded}
      actions={
        showCopyButtons ? (
          <>
            <CopyButton
              getContent={getJsonContent}
              label={t('copy_json_short', { defaultValue: 'JSON' })}
              icon={<FileJson size={size === 'md' ? 12 : 10} />}
              size={size}
            />
            <CopyButton
              getContent={getMarkdownContent}
              label={t('copy_markdown_short', { defaultValue: 'Markdown' })}
              icon={<FileText size={size === 'md' ? 12 : 10} />}
              size={size}
            />
          </>
        ) : undefined
      }
    >
      <PluginResultPanel results={results} />
    </ExpandableSection>
  );
}
