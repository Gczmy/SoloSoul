import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { formatBytes } from '@/lib/utils';
import { AttachmentLimitsInfo } from './AttachmentLimitsInfo';
import { WarningCancelButton } from './WarningCancelButton';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { ObjectSelectionTree } from '@/components/transfer/ObjectSelectionTree';
import { TransferButton } from '@/components/transfer/TransferButton';
import type { ExportEstimate } from '@/types/exportImport';

interface PageGroup {
  sectionType: string;
  pageName: string;
  objectCount: number;
  objects: ObjectSummary[];
}

interface ObjectSummary {
  id: string;
  name: string;
  collectionType: string;
  sectionType: string;
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
  tags: string[];
  hasAttachments?: boolean;
}

interface AttachmentInfo {
  id: string;
  fileName: string;
  sizeBytes: number;
}

interface ExportSectionProps {
  pageGroups: PageGroup[];
  selectedPageIds: Set<string>;
  selectedObjectIds: Set<string>;
  expandedPages: Set<string>;
  exportPassword: string;
  exportPasswordConfirm: string;
  exportHint: string;
  savePath: string | null;
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

export function ExportSection({
  pageGroups,
  selectedPageIds,
  selectedObjectIds,
  expandedPages,
  exportPassword,
  exportPasswordConfirm,
  exportHint,
  savePath,
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

      {/* Page & Object tree */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('settings:select_objects')}
        </h3>
        <ObjectSelectionTree
          pageGroups={pageGroups}
          selectedPageIds={selectedPageIds}
          expandedPages={expandedPages}
          expandedObjects={expandedObjects}
          selectedAttachmentIds={selectedAttachmentIds}
          objectAttachments={objectAttachments}
          totalSelected={totalSelected}
          // 仅真实包含（未软删）附件的对象显示展开图标；无附件对象不再展示「无附件」空面板
          showAttachmentExpand={(obj) => includeAttachments && obj.hasAttachments === true}
          isObjectSelected={(id) => selectedObjectIds.has(id)}
          onTogglePage={onTogglePage}
          onToggleObject={onToggleObject}
          onToggleObjectExpanded={onToggleObjectExpanded}
          onToggleAttachment={onToggleAttachment}
          onToggleExpandedPage={onToggleExpandedPage}
          onSelectAll={onSelectAllExport}
        />
      </Card>

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

      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('settings:export_options')}
        </h3>
        <div style={{ padding: '4px 0' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <label
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                cursor: 'pointer',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              <SelectCheckbox
                checked={includeAttachments}
                onChange={(v) => onSetIncludeAttachments(v)}
              />
              {t('settings:include_attachments')}
            </label>
            <AttachmentLimitsInfo />
          </div>
          <div
            style={{
              paddingLeft: 24,
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              marginTop: 2,
            }}
          >
            {t('settings:include_attachments_desc')}
          </div>
        </div>
        <div style={{ padding: '4px 0' }}>
          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              cursor: 'pointer',
              fontSize: 'var(--text-body-sm)',
            }}
          >
            <SelectCheckbox
              checked={includePreferences}
              onChange={(v) => onSetIncludePreferences(v)}
            />
            {t('settings:include_preferences')}
          </label>
          <div
            style={{
              paddingLeft: 24,
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              marginTop: 2,
            }}
          >
            {t('settings:include_preferences_desc')}
          </div>
        </div>
        <div style={{ padding: '4px 0' }}>
          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              cursor: 'pointer',
              fontSize: 'var(--text-body-sm)',
            }}
          >
            <SelectCheckbox
              checked={includeBehavioral}
              onChange={(v) => onSetIncludeBehavioral(v)}
            />
            {t('settings:include_behavioral')}
          </label>
          <div
            style={{
              paddingLeft: 24,
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              marginTop: 2,
            }}
          >
            {t('settings:include_behavioral_desc')}
          </div>
        </div>
      </Card>

      {/* Save path */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('common:export_path')}
        </h3>
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

      {/* Encryption */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('settings:encryption')}
        </h3>

        {hasSensitiveData && (
          <div
            style={{
              marginBottom: 10,
              padding: '8px 12px',
              background: 'var(--warning-subtle)',
              borderRadius: 6,
              fontSize: 'var(--text-caption)',
              color: 'var(--warning)',
              border: '1px solid var(--warning)',
            }}
          >
            {t('settings:sensitive_export_warning')}
          </div>
        )}

        <SecurePasswordInput
          value={exportPassword}
          onChange={onSetExportPassword}
          placeholder={t('common:password_placeholder')}
          showHintButton={false}
          onEnter={onExport}
        />
        <div style={{ marginTop: 8 }}>
          <SecurePasswordInput
            value={exportPasswordConfirm}
            onChange={(v) => onSetExportPasswordConfirm(v)}
            placeholder={t('settings:confirm_password')}
            showHintButton={false}
            onEnter={onExport}
          />
        </div>
        {exportPassword && exportPasswordConfirm && exportPassword !== exportPasswordConfirm && (
          <div style={{ marginTop: 4, fontSize: 'var(--text-caption)', color: 'var(--danger)' }}>
            {t('settings:password_mismatch')}
          </div>
        )}
        <div style={{ marginTop: 8 }}>
          <input
            type="text"
            value={exportHint}
            onChange={(e) => onSetExportHint(e.target.value)}
            placeholder={t('common:password_hint')}
            maxLength={200}
            className="interactive-field"
            style={{
              width: '100%',
              padding: '10px 14px',
              fontSize: 'var(--text-body)',
              borderWidth: 1,
              borderStyle: 'solid',
              borderRadius: 8,
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontFamily: 'inherit',
              outline: 'none',
            }}
          />
        </div>
      </Card>

      {/* Weak password confirmation dialog */}
      {showWeakPasswordWarning && (
        <div
          style={{
            padding: '12px 16px',
            borderRadius: 8,
            background: 'var(--warning-subtle)',
            border: '1px solid var(--warning)',
            fontSize: 'var(--text-body-sm)',
            color: 'var(--warning)',
          }}
        >
          <p style={{ marginBottom: 8, fontWeight: 600 }}>{t('settings:weak_password_title')}</p>
          <p style={{ marginBottom: 10 }}>{t('settings:weak_password_confirm')}</p>
          <div style={{ display: 'flex', gap: 8 }}>
            <WarningCancelButton onClick={() => onSetShowWeakPasswordWarning(false)}>
              {t('common:cancel')}
            </WarningCancelButton>
            <TransferButton variant="warning" onClick={onSetWeakPasswordExport}>
              {t('settings:export_anyway')}
            </TransferButton>
          </div>
        </div>
      )}

      {/* Password hint risk confirmation dialog */}
      {showHintWarning && (
        <div
          style={{
            padding: '12px 16px',
            borderRadius: 8,
            background: 'var(--warning-subtle)',
            border: '1px solid var(--warning)',
            fontSize: 'var(--text-body-sm)',
            color: 'var(--warning)',
          }}
        >
          <p style={{ marginBottom: 8, fontWeight: 600 }}>
            {t('settings:hint_contains_password_title')}
          </p>
          <p style={{ marginBottom: 10 }}>{t('settings:hint_contains_password_confirm')}</p>
          <div style={{ display: 'flex', gap: 8 }}>
            <WarningCancelButton onClick={() => onSetShowHintWarning(false)}>
              {t('common:cancel')}
            </WarningCancelButton>
            <TransferButton variant="warning" onClick={onSetShowHintWarningAndExport}>
              {t('settings:export_anyway')}
            </TransferButton>
          </div>
        </div>
      )}

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
