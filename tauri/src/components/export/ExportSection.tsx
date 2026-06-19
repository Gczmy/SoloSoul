import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { Paperclip } from 'lucide-react';
import { formatBytes } from '@/lib/format';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { AttachmentLimitsInfo }  from './AttachmentLimitsInfo';
import { WarningCancelButton } from './WarningCancelButton';

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
}

interface AttachmentInfo {
  id: string;
  fileName: string;
  sizeBytes: number;
}

interface ExportEstimate {
  objectCount: number;
  attachmentCount: number;
  attachmentSelectedCount: number;
  estimatedBytes: number;
}

type PasswordStrength = 'none' | 'weak' | 'medium' | 'strong';

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
  showWeakWarning: boolean;
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
  pwStrength: PasswordStrength;
  pwStrengthLabel: Record<PasswordStrength, string>;
  hasSensitiveData: boolean;
  allTags: string[];
  totalSelected: number;
  onTogglePage: (sectionType: string, objectIds: string[]) => void;
  onToggleObject: (id: string, sectionType: string, allIdsInGroup: string[]) => void;
  onToggleObjectExpanded: (objectId: string) => void;
  onToggleAttachment: (attId: string, objectId: string, sectionType: string, allIdsInGroup: string[]) => void;
  onSetExportPassword: (v: string) => void;
  onSetExportPasswordConfirm: (v: string) => void;
  onSetExportHint: (v: string) => void;
  onSetSavePath: (v: string | null) => void;
  onExport: () => void;
  onSetShowWeakWarning: (v: boolean) => void;
  onSetShowHintWarning: (v: boolean) => void;
  onSetSelectedTags: (updater: (prev: Set<string>) => Set<string>) => void;
  onSetIncludeAttachments: (v: boolean) => void;
  onSetIncludePreferences: (v: boolean) => void;
  onSetIncludeBehavioral: (v: boolean) => void;
  onSetShowWeakWarningAndExport: () => void;
  onToggleExpandedPage: (sectionType: string) => void;
  onSetShowHintWarningAndExport: () => void;
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
  showWeakWarning,
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
  pwStrength,
  pwStrengthLabel,
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
  onSetShowWeakWarning,
  onSetShowHintWarning,
  onSetSelectedTags,
  onSetIncludeAttachments,
  onSetIncludePreferences,
  onSetIncludeBehavioral,
  onSetShowWeakWarningAndExport,
  onToggleExpandedPage,
  onSetShowHintWarningAndExport,
}: ExportSectionProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <>
      <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
        {t('settings:export_desc')}
      </p>

      {/* Page & Object tree */}
      <Card>
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
          {t('settings:select_objects')}
        </h3>
        {pageGroups.length === 0 ? (
          <p style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>{t('common:no_data')}</p>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {pageGroups.map((group) => {
              const allIds = group.objects.map((o) => o.id);
              const pageChecked = selectedPageIds.has(group.sectionType);
              const someChecked =
                !pageChecked && allIds.some((id) => selectedObjectIds.has(id));
              const expanded = expandedPages.has(group.sectionType);
              return (
                <div key={group.sectionType}>
                  {/* Page row */}
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      padding: '8px 0',
                      cursor: 'pointer',
                      userSelect: 'none',
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={pageChecked}
                      ref={(el) => {
                        if (el) el.indeterminate = someChecked && !pageChecked;
                      }}
                      onChange={() => onTogglePage(group.sectionType, allIds)}
                      style={{ accentColor: 'var(--accent-primary)' }}
                    />
                    <span
                      onClick={() => {
                        onToggleExpandedPage(group.sectionType);
                        // Toggle expanded state via callback to parent
                      }}
                      style={{
                        fontSize: 14,
                        fontWeight: 600,
                        flex: 1,
                        display: 'flex',
                        alignItems: 'center',
                        gap: 4,
                      }}
                    >
                      <span
                        style={{
                          transform: expanded ? 'rotate(90deg)' : 'none',
                          transition: 'transform 0.15s',
                          fontSize: 10,
                        }}
                      >
                        ▶
                      </span>
                      {t(`navigation:${group.sectionType}`, group.pageName)}
                    </span>
                    <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                      {t('common:object_count', { n: group.objectCount })}
                    </span>
                  </div>

                  {/* Object rows (collapsible) */}
                  {expanded &&
                    group.objects.map((obj) => (
                      <div key={obj.id}>
                        <label
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 8,
                            padding: '4px 0 4px 28px',
                            cursor: 'pointer',
                          }}
                        >
                          <input
                            type="checkbox"
                            checked={selectedObjectIds.has(obj.id)}
                            onChange={() => onToggleObject(obj.id, group.sectionType, allIds)}
                            style={{ accentColor: 'var(--accent-primary)' }}
                          />
                          <span style={{ fontSize: 13, flex: 1 }}>{obj.name}</span>
                          <SensitivityBadge
                            level={obj.sensitivityLevel as SensitivityLevel}
                          />
                          {includeAttachments && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                onToggleObjectExpanded(obj.id);
                              }}
                              style={{
                                fontSize: 10,
                                background: 'none',
                                border: 'none',
                                cursor: 'pointer',
                                padding: '0 4px',
                                transform: expandedObjects.has(obj.id)
                                  ? 'rotate(90deg)'
                                  : 'none',
                                transition: 'transform 0.15s',
                                color: 'var(--text-tertiary)',
                              }}
                            >
                              ▶
                            </button>
                          )}
                        </label>
                        {includeAttachments && expandedObjects.has(obj.id) && (
                          <div style={{ paddingLeft: 52, paddingBottom: 4 }}>
                            {(objectAttachments.get(obj.id) || []).length === 0 ? (
                              <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                                {t('settings:no_attachments', 'No attachments')}
                              </span>
                            ) : (
                              <>
                                <div
                                  style={{
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: 4,
                                    padding: '2px 0',
                                    fontSize: 11,
                                    color: 'var(--text-tertiary)',
                                    borderBottom: '1px solid var(--border-subtle)',
                                    marginBottom: 2,
                                  }}
                                >
                                  <Paperclip size={10} />
                                  <span>
                                    {t('settings:attachments_label', 'Attachments')} (
                                    {(objectAttachments.get(obj.id) || []).length})
                                  </span>
                                </div>
                                {(objectAttachments.get(obj.id) || []).map((att) => (
                                  <label
                                    key={att.id}
                                    style={{
                                      display: 'flex',
                                      alignItems: 'center',
                                      gap: 6,
                                      padding: '2px 0 2px 16px',
                                      cursor: 'pointer',
                                    }}
                                  >
                                    <input
                                      type="checkbox"
                                      checked={selectedAttachmentIds.has(att.id)}
                                      onChange={() =>
                                        onToggleAttachment(
                                          att.id,
                                          obj.id,
                                          group.sectionType,
                                          allIds,
                                        )
                                      }
                                      style={{ accentColor: 'var(--accent-primary)' }}
                                    />
                                    <Paperclip
                                      size={10}
                                      style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
                                    />
                                    <span style={{ fontSize: 12, flex: 1 }}>
                                      {att.fileName}
                                    </span>
                                    <span
                                      style={{ fontSize: 11, color: 'var(--text-tertiary)' }}
                                    >
                                      {formatBytes(att.sizeBytes)}
                                    </span>
                                  </label>
                                ))}
                              </>
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                </div>
              );
            })}
          </div>
        )}
      </Card>

      {/* Tag filter */}
      {allTags.length > 0 && (
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
            {t('settings:filter_by_tags')}
          </h3>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
            {allTags.map((tag) => {
              const isSelected = selectedTags.has(tag);
              return (
                <button
                  key={tag}
                  onClick={() => onSetSelectedTags((prev) => {
                    const next = new Set(prev);
                    if (next.has(tag)) next.delete(tag);
                    else next.add(tag);
                    return next;
                  })}
                  style={{
                    fontSize: 12,
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
              fontSize: 13,
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
        </Card>
      )}

      <Card>
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
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
                fontSize: 13,
              }}
            >
              <input
                type="checkbox"
                checked={includeAttachments}
                onChange={() => onSetIncludeAttachments(!includeAttachments)}
                style={{ accentColor: 'var(--accent-primary)' }}
              />
              {t('settings:include_attachments')}
            </label>
            <AttachmentLimitsInfo />
          </div>
          <div
            style={{
              paddingLeft: 24,
              fontSize: 11,
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
              fontSize: 13,
            }}
          >
            <input
              type="checkbox"
              checked={includePreferences}
              onChange={() => onSetIncludePreferences(!includePreferences)}
              style={{ accentColor: 'var(--accent-primary)' }}
            />
            {t('settings:include_preferences')}
          </label>
          <div
            style={{
              paddingLeft: 24,
              fontSize: 11,
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
              fontSize: 13,
            }}
          >
            <input
              type="checkbox"
              checked={includeBehavioral}
              onChange={() => onSetIncludeBehavioral(!includeBehavioral)}
              style={{ accentColor: 'var(--accent-primary)' }}
            />
            {t('settings:include_behavioral')}
          </label>
          <div
            style={{
              paddingLeft: 24,
              fontSize: 11,
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
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
          {t('common:export_path')}
        </h3>
        <div style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 8 }}>
          {savePath || t('settings:no_file_selected')}
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={async () => {
            const { save } = await import('@tauri-apps/plugin-dialog');
            const fp = await save({
              filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
              defaultPath: `solosoul_export_${Date.now()}.solosoul`,
            });
            if (fp) onSetSavePath(fp);
          }}
        >
          {t('common:browse')}
        </Button>
      </Card>

      {/* Encryption */}
      <Card>
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
          {t('settings:encryption')}
        </h3>

        {hasSensitiveData && (
          <div
            style={{
              marginBottom: 10,
              padding: '8px 12px',
              background: '#fff3e0',
              borderRadius: 6,
              fontSize: 12,
              color: '#663c00',
              border: '1px solid #ffcc80',
            }}
          >
            {t('settings:sensitive_export_warning')}
          </div>
        )}

        <SecurePasswordInput
          value={exportPassword}
          onChange={(v) => {
            onSetExportPassword(v);
            onSetShowWeakWarning(false);
          }}
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
        {exportPassword &&
          exportPasswordConfirm &&
          exportPassword !== exportPasswordConfirm && (
            <div style={{ marginTop: 4, fontSize: 12, color: '#d32f2f' }}>
              {t('settings:password_mismatch')}
            </div>
          )}
        {exportPassword && (
          <div style={{ marginTop: 6, fontSize: 12, color: 'var(--text-secondary)' }}>
            {t('settings:password_strength')}:{' '}
            <span
              style={{
                color:
                  pwStrength === 'weak'
                    ? '#d32f2f'
                    : pwStrength === 'medium'
                      ? '#e68a00'
                      : '#2e7d32',
              }}
            >
              {pwStrengthLabel[pwStrength]}
            </span>
            {pwStrength === 'weak' && (
              <span style={{ marginLeft: 8, color: '#d32f2f', fontSize: 11 }}>
                {t('settings:password_weak_warning')}
              </span>
            )}
          </div>
        )}
        <div style={{ marginTop: 8 }}>
          <input
            type="text"
            value={exportHint}
            onChange={(e) => onSetExportHint(e.target.value)}
            placeholder={t('common:password_hint')}
            maxLength={200}
            style={{
              width: '100%',
              padding: '10px 14px',
              fontSize: 14,
              border: '1px solid var(--border-subtle)',
              borderRadius: 8,
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontFamily: 'inherit',
              outline: 'none',
            }}
          />
        </div>
      </Card>

      {/* Password hint risk confirmation dialog */}
      {showHintWarning && (
        <div
          style={{
            padding: '12px 16px',
            borderRadius: 8,
            background: '#fff3e0',
            border: '1px solid #ffcc80',
            fontSize: 13,
            color: '#663c00',
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
            <Button
              size="sm"
              onClick={onSetShowHintWarningAndExport}
            >
              {t('settings:export_anyway')}
            </Button>
          </div>
        </div>
      )}

      {/* Weak password confirmation dialog */}
      {showWeakWarning && (
        <div
          style={{
            padding: '12px 16px',
            borderRadius: 8,
            background: '#fff3e0',
            border: '1px solid #ffcc80',
            fontSize: 13,
            color: '#663c00',
          }}
        >
          <p style={{ marginBottom: 8, fontWeight: 600 }}>
            {t('settings:weak_password_title')}
          </p>
          <p style={{ marginBottom: 10 }}>{t('settings:weak_password_confirm')}</p>
          <div style={{ display: 'flex', gap: 8 }}>
            <WarningCancelButton onClick={() => onSetShowWeakWarning(false)}>
              {t('common:cancel')}
            </WarningCancelButton>
            <Button
              size="sm"
              onClick={onSetShowWeakWarningAndExport}
            >
              {t('settings:export_anyway')}
            </Button>
          </div>
        </div>
      )}

      <Button
        onClick={onExport}
        loading={isExporting}
        disabled={totalSelected === 0 || !exportPassword || !savePath}
      >
        {t('settings:export_selected')} ({totalSelected})
      </Button>
    </>
  );
}
