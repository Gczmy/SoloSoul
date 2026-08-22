import { useTranslation } from 'react-i18next';
import { Cloud } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { formatBytes } from '@/lib/utils';
import { TransferButton } from '@/components/transfer/TransferButton';
import type { AttachmentInfo, ExportEstimate, CloudTargetInfo } from '@/types/exportImport';

import { ExportTreeCard, type PageGroup } from './ExportTreeCard';
import { ExportOptionsCard } from './ExportOptionsCard';
import { ExportEncryptionCard } from './ExportEncryptionCard';
import { ExportWarningDialogs } from './ExportWarningDialogs';

export type { PageGroup } from './ExportTreeCard';

interface ExportSectionProps {
  pageGroups: PageGroup[];
  selectedPageIds: Set<string>;
  selectedObjectIds: Set<string>;
  expandedPages: Set<string>;
  exportPassword: string;
  exportPasswordConfirm: string;
  exportHint: string;
  savePath: string | null;
  /** Phase 1 云打包：检测到的云盘同步目录快捷目标（桌面端；移动端为空）。 */
  cloudTargets: CloudTargetInfo[];
  isExporting: boolean;
  showHintWarning: boolean;
  selectedTags: Set<string>;
  includeAttachments: boolean;
  selectedAttachmentIds: Set<string>;
  objectAttachments: Map<string, AttachmentInfo[]>;
  expandedObjects: Set<string>;
  includePreferences: boolean;
  includeBehavioral: boolean;
  exportEstimate: ExportEstimate | null;
  estimating: boolean;
  hasSensitiveData: boolean;
  allTags: string[];
  totalSelected: number;
  onTogglePage: (sectionType: string, objectIds: string[]) => void;
  onToggleObject: (id: string, sectionType: string, allIdsInGroup: string[]) => void;
  onToggleObjectExpanded: (objectId: string) => void;
  onToggleAttachment: (
    attId: string,
    objectId: string,
    sectionType: string,
    allIdsInGroup: string[],
  ) => void;
  onSetExportPassword: (v: string) => void;
  onSetExportPasswordConfirm: (v: string) => void;
  onSetExportHint: (v: string) => void;
  onSetSavePath: (v: string | null) => void;
  onExport: () => void;
  onSetShowHintWarning: (v: boolean) => void;
  onSetSelectedTags: (updater: (prev: Set<string>) => Set<string>) => void;
  onSetIncludeAttachments: (v: boolean) => void;
  onSetIncludePreferences: (v: boolean) => void;
  onSetIncludeBehavioral: (v: boolean) => void;
  onToggleExpandedPage: (sectionType: string) => void;
  onSelectAllExport: (selectAll: boolean) => void;
  showWeakPasswordWarning: boolean;
  onSetShowWeakPasswordWarning: (v: boolean) => void;
  onSetShowHintWarningAndExport: () => void;
  onSetWeakPasswordExport: () => void;
}

/**
 * 导出配置区 — P046 拆分后为纯组合层：
 * 页面/对象树（ExportTreeCard）、导出选项（ExportOptionsCard）、
 * 加密输入（ExportEncryptionCard）、风险确认（ExportWarningDialogs）均为独立展示子组件；
 * 本组件保留标签筛选、大小预估、保存路径与导出按钮。
 */
export function ExportSection({
  pageGroups,
  selectedPageIds,
  selectedObjectIds,
  expandedPages,
  exportPassword,
  exportPasswordConfirm,
  exportHint,
  savePath,
  cloudTargets,
  isExporting,
  showHintWarning,
  selectedTags,
  includeAttachments,
  selectedAttachmentIds,
  objectAttachments,
  expandedObjects,
  includePreferences,
  includeBehavioral,
  exportEstimate,
  estimating,
  hasSensitiveData,
  allTags,
  totalSelected,
  onTogglePage,
  onToggleObject,
  onToggleObjectExpanded,
  onToggleAttachment,
  onSetExportPassword,
  onSetExportPasswordConfirm,
  onSetExportHint,
  onSetSavePath,
  onExport,
  onSetShowHintWarning,
  onSetSelectedTags,
  onSetIncludeAttachments,
  onSetIncludePreferences,
  onSetIncludeBehavioral,
  showWeakPasswordWarning,
  onSetShowWeakPasswordWarning,
  onSelectAllExport,
  onToggleExpandedPage,
  onSetShowHintWarningAndExport,
  onSetWeakPasswordExport,
}: ExportSectionProps) {
  const { t, i18n } = useTranslation(['settings', 'common']);

  /** 按当前语言本地化连接名称列表（zh: 「、」；en: ", and"） */
  const formatNameList = (names: string[]) =>
    new Intl.ListFormat(i18n.language, { style: 'long', type: 'conjunction' }).format(names);

  return (
    <>
      <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
        {t('settings:export_desc')}
      </p>

      {/* Page & Object tree（P046 拆分：ExportTreeCard） */}
      <ExportTreeCard
        pageGroups={pageGroups}
        selectedPageIds={selectedPageIds}
        selectedObjectIds={selectedObjectIds}
        expandedPages={expandedPages}
        expandedObjects={expandedObjects}
        selectedAttachmentIds={selectedAttachmentIds}
        objectAttachments={objectAttachments}
        totalSelected={totalSelected}
        includeAttachments={includeAttachments}
        onTogglePage={onTogglePage}
        onToggleObject={onToggleObject}
        onToggleObjectExpanded={onToggleObjectExpanded}
        onToggleAttachment={onToggleAttachment}
        onToggleExpandedPage={onToggleExpandedPage}
        onSelectAllExport={onSelectAllExport}
        t={t}
      />

      {/* Tag filter */}
      {allTags.length > 0 && (
        <Card>
          <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
            {t('settings:filter_by_tags')}
          </h3>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
            {allTags.map((tag) => {
              const isSelected = selectedTags.has(tag);
              return (
                <button
                  key={tag}
                  onClick={() =>
                    onSetSelectedTags((prev) => {
                      const next = new Set(prev);
                      if (next.has(tag)) next.delete(tag);
                      else next.add(tag);
                      return next;
                    })
                  }
                  style={{
                    fontSize: 'var(--text-caption)',
                    padding: '4px 10px',
                    borderRadius: 12,
                    border: '1px solid var(--border-subtle)',
                    background: isSelected ? 'var(--accent-primary)' : 'var(--bg-elevated)',
                    color: isSelected ? 'white' : 'var(--text-primary)',
                    cursor: 'pointer',
                  }}
                >
                  {tag}
                </button>
              );
            })}
          </div>
        </Card>
      )}

      {/* Export size estimate */}
      {totalSelected > 0 && (
        <Card>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              fontSize: 'var(--text-body-sm)',
              padding: '4px 0',
            }}
          >
            <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
              {t('settings:export_estimate_label', 'Export file estimated size')}
            </span>
            <span style={{ color: 'var(--text-secondary)' }}>
              {estimating
                ? t('settings:estimating')
                : exportEstimate
                  ? `${t('settings:objects_count', { n: exportEstimate.objectCount })}` +
                    (exportEstimate.attachmentSelectedCount > 0
                      ? ` + ${t('settings:attachments_count', { n: exportEstimate.attachmentSelectedCount })}`
                      : '') +
                    ` · ${formatBytes(exportEstimate.estimatedBytes)}`
                  : ''}
            </span>
          </div>
          {/* 随导出打包的模板快照清单：让导出者明确知道哪些模板会被导出 */}
          {!estimating && exportEstimate && exportEstimate.templateCount > 0 && (
            <div
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-secondary)',
                padding: '4px 0',
                borderTop: '1px solid var(--border-subtle)',
              }}
            >
              {t('settings:export_templates_line', {
                count: exportEstimate.templateCount,
                names:
                  formatNameList(exportEstimate.templateNames.slice(0, 5)) +
                  (exportEstimate.templateCount > 5
                    ? t('settings:export_templates_truncated_suffix')
                    : ''),
              })}
            </div>
          )}
        </Card>
      )}

      {/* Export options（P046 拆分：ExportOptionsCard） */}
      <ExportOptionsCard
        includeAttachments={includeAttachments}
        includePreferences={includePreferences}
        includeBehavioral={includeBehavioral}
        onSetIncludeAttachments={onSetIncludeAttachments}
        onSetIncludePreferences={onSetIncludePreferences}
        onSetIncludeBehavioral={onSetIncludeBehavioral}
        t={t}
      />

      {/* Save path */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('common:export_path')}
        </h3>
        {cloudTargets.length > 0 && (
          <div style={{ marginBottom: 10 }}>
            <div
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                marginBottom: 6,
              }}
            >
              {t('settings:cloud_save_targets')}
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
              {cloudTargets.map((target) => {
                const sep = target.path.includes('\\') ? '\\' : '/';
                const cloudPath = `${target.path}${sep}SoloSoul${sep}solosoul_export_${Date.now()}.solosoul`;
                const isActive = !!savePath && savePath.startsWith(target.path);
                return (
                  <button
                    key={target.path}
                    type="button"
                    onClick={() => onSetSavePath(cloudPath)}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 4,
                      padding: '4px 10px',
                      borderRadius: 999,
                      border: `1px solid ${isActive ? 'var(--accent)' : 'var(--border)'}`,
                      background: isActive ? 'var(--accent-soft, rgba(0,0,0,0.05))' : 'transparent',
                      color: 'var(--text-primary)',
                      fontSize: 'var(--text-body-sm)',
                      cursor: 'pointer',
                    }}
                  >
                    <Cloud size={14} aria-hidden />
                    {target.name}
                  </button>
                );
              })}
            </div>
          </div>
        )}
        <div
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 8,
            // Android content:// URI 很长，折行防止溢出卡片
            wordBreak: 'break-all',
          }}
        >
          {savePath || t('settings:no_file_selected')}
        </div>
        <TransferButton
          onClick={async () => {
            const { saveWithPause } = await import('@/lib/dialog');
            const fp = await saveWithPause({
              filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
              defaultPath: `solosoul_export_${Date.now()}.solosoul`,
            });
            if (fp) onSetSavePath(fp);
          }}
        >
          {t('common:browse')}
        </TransferButton>
      </Card>

      {/* Encryption（P046 拆分：ExportEncryptionCard） */}
      <ExportEncryptionCard
        exportPassword={exportPassword}
        exportPasswordConfirm={exportPasswordConfirm}
        exportHint={exportHint}
        hasSensitiveData={hasSensitiveData}
        onSetExportPassword={onSetExportPassword}
        onSetExportPasswordConfirm={onSetExportPasswordConfirm}
        onSetExportHint={onSetExportHint}
        onExport={onExport}
        t={t}
      />

      {/* Risk confirmations（P046 拆分：ExportWarningDialogs） */}
      <ExportWarningDialogs
        showWeakPasswordWarning={showWeakPasswordWarning}
        showHintWarning={showHintWarning}
        onSetShowWeakPasswordWarning={onSetShowWeakPasswordWarning}
        onSetShowHintWarning={onSetShowHintWarning}
        onSetWeakPasswordExport={onSetWeakPasswordExport}
        onSetShowHintWarningAndExport={onSetShowHintWarningAndExport}
        t={t}
      />

      <TransferButton
        variant="accent"
        onClick={onExport}
        disabled={totalSelected === 0 || !exportPassword || !savePath}
        busy={isExporting}
      >
        {isExporting
          ? t('common:loading', { defaultValue: '...' })
          : `${t('settings:export_selected')} (${totalSelected})`}
      </TransferButton>
    </>
  );
}
