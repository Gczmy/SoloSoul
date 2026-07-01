import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { Paperclip } from 'lucide-react';
import { formatBytes } from '@/lib/format';
import { ICON_SIZE } from '@/lib/iconSizes';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import type {
  AttachmentImportInfo,
  ImportStrategy,
  ImportPreview,
  DecryptedImportPreview,
  ObjectSummary,
} from '@/types/exportImport';

interface ImportSectionProps {
  importPath: string;
  importPreview: ImportPreview | null;
  importPw: string;
  decryptedPreview: DecryptedImportPreview | null;
  isPreviewing: boolean;
  isDecrypting: boolean;
  isImporting: boolean;
  importStrategy: ImportStrategy;
  importSelections: Map<string, boolean>;
  showStrategySelector: boolean;
  importSelectedPageIds: Set<string>;
  importSelectedAttachmentIds: Set<string>;
  importExpandedPages: Set<string>;
  importExpandedObjects: Set<string>;
  importTotalSelected: number;
  onSetImportPath: (v: string) => void;
  onSetImportPreview: (v: ImportPreview | null) => void;
  onSetDecryptedPreview: (v: DecryptedImportPreview | null) => void;
  onSetImportPw: (v: string) => void;
  onSetShowStrategySelector: (v: boolean) => void;
  onPreview: () => void;
  onDecrypt: () => void;
  onImport: () => void;
  onToggleSelection: (id: string) => void;
  onToggleImportPage: (sectionType: string, objectIds: string[]) => void;
  onToggleImportAttachment: (attId: string) => void;
  onToggleExpandedImportPage: (sectionType: string) => void;
  onToggleImportObjectExpanded: (objectId: string) => void;
  onSetStrategy: (s: ImportStrategy) => void;
}

/** Group decrypted preview objects by section_type into pages */
function groupIntoPages(objects: ObjectSummary[]) {
  const map = new Map<string, { sectionType: string; objects: ObjectSummary[] }>();
  for (const obj of objects) {
    const st = obj.sectionType || 'uncategorized';
    let group = map.get(st);
    if (!group) {
      group = { sectionType: st, objects: [] };
      map.set(st, group);
    }
    group.objects.push(obj);
  }
  return Array.from(map.values());
}

export function ImportSection({
  importPath,
  importPreview,
  importPw,
  decryptedPreview,
  isPreviewing,
  isDecrypting,
  isImporting,
  importStrategy,
  importSelections,
  showStrategySelector,
  importSelectedPageIds,
  importSelectedAttachmentIds,
  importExpandedPages,
  importExpandedObjects,
  importTotalSelected,
  onSetImportPath,
  onSetImportPreview,
  onSetDecryptedPreview,
  onSetImportPw,
  onSetShowStrategySelector,
  onPreview,
  onDecrypt,
  onImport,
  onToggleSelection,
  onToggleImportPage,
  onToggleImportAttachment,
  onToggleExpandedImportPage,
  onToggleImportObjectExpanded,
  onSetStrategy,
}: ImportSectionProps) {
  const { t } = useTranslation(['settings', 'common', 'navigation']);

  // Build pages from decrypted objects
  const importPageGroups = useMemo(
    () => (decryptedPreview ? groupIntoPages(decryptedPreview.objects) : []),
    [decryptedPreview],
  );

  // Build a lookup set for conflict object IDs
  const conflictIds = useMemo(
    () => new Set(decryptedPreview?.conflicts.map((c) => c.objectId) ?? []),
    [decryptedPreview],
  );

  // Attachments grouped by object ID
  const attachmentsByObject = useMemo(() => {
    const map = new Map<string, AttachmentImportInfo[]>();
    if (decryptedPreview) {
      for (const att of decryptedPreview.attachments) {
        let list = map.get(att.objectId);
        if (!list) {
          list = [];
          map.set(att.objectId, list);
        }
        list.push(att);
      }
    }
    return map;
  }, [decryptedPreview]);

  return (
    <>
      <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
        {t('settings:import_desc')}
      </p>

      {/* File selector */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('settings:select_file')}
        </h3>
        <div
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 8,
          }}
        >
          {importPath || t('settings:no_file_selected')}
        </div>
        <button
          type="button"
          onClick={async () => {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selected = await open({
              filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
              multiple: false,
            });
            if (selected) {
              onSetImportPath(selected as string);
              onSetImportPreview(null);
              onSetDecryptedPreview(null);
              onSetImportPw('');
              onSetShowStrategySelector(false);
            }
          }}
          style={{
            fontSize: 'var(--text-caption)',
            padding: '6px 12px',
            borderRadius: 6,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-toolbar)',
            color: 'var(--text-primary)',
            cursor: 'pointer',
            fontFamily: 'inherit',
            fontWeight: 500,
            transition: 'all 0.15s ease',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background =
              'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
            e.currentTarget.style.borderColor = 'var(--accent-primary)';
            e.currentTarget.style.color = 'var(--accent-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'var(--bg-toolbar)';
            e.currentTarget.style.borderColor = 'var(--border-subtle)';
            e.currentTarget.style.color = 'var(--text-primary)';
          }}
        >
          {t('settings:select_file')}
        </button>
        {importPath && !importPreview && (
          <div style={{ marginTop: 8 }}>
            <button
              type="button"
              onClick={onPreview}
              disabled={isPreviewing}
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: isPreviewing
                  ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                  : 'var(--bg-toolbar)',
                color: isPreviewing ? 'var(--accent-primary)' : 'var(--text-primary)',
                cursor: isPreviewing ? 'default' : 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
                opacity: isPreviewing ? 0.6 : 1,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                if (!isPreviewing) {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isPreviewing) {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }
              }}
            >
              {isPreviewing ? t('common:loading', { defaultValue: '...' }) : t('settings:preview')}
            </button>
          </div>
        )}
      </Card>

      {/* Parsed manifest preview */}
      {importPreview && (
        <Card>
          <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
            {t('settings:import_preview')}
          </h3>
          <div
            style={{
              fontSize: 'var(--text-body-sm)',
              display: 'flex',
              flexDirection: 'column',
              gap: 6,
            }}
          >
            <p>
              {t('settings:version')}: {importPreview.version}
            </p>
            <p>
              {t('settings:export_time')}: {importPreview.exportTime || t('settings:unknown')}
            </p>
            <p>{t('settings:objects_count', { n: importPreview.objectCount })}</p>
            {importPreview.hasAttachments && (
              <p style={{ color: 'var(--accent-primary)' }}>{t('settings:includes_attachments')}</p>
            )}
            {importPreview.extraFiles.length > 0 &&
              importPreview.extraFiles.includes('preferences.enc') && (
                <p style={{ color: 'var(--accent-primary)' }}>
                  {t('settings:includes_preferences')}
                </p>
              )}
          </div>

          {importPreview.passwordHint && (
            <div
              style={{
                marginTop: 8,
                padding: '8px 12px',
                background: 'var(--bg-elevated-hover)',
                borderRadius: 6,
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
              }}
            >
              {t('settings:password_hint_label')}: {importPreview.passwordHint}
            </div>
          )}

          <div style={{ marginTop: 12 }}>
            <SecurePasswordInput
              value={importPw}
              onChange={(v) => onSetImportPw(v)}
              placeholder={t('common:password_placeholder')}
              showHintButton={false}
              onEnter={onDecrypt}
            />
          </div>
          {!decryptedPreview && (
            <div style={{ marginTop: 8 }}>
              <button
                type="button"
                onClick={onDecrypt}
                disabled={!importPw || isDecrypting}
                style={{
                  fontSize: 'var(--text-caption)',
                  padding: '6px 12px',
                  borderRadius: 6,
                  border: '1px solid var(--border-subtle)',
                  background: isDecrypting
                    ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                    : 'var(--bg-toolbar)',
                  color: isDecrypting ? 'var(--accent-primary)' : 'var(--text-primary)',
                  cursor: !importPw || isDecrypting ? 'default' : 'pointer',
                  fontFamily: 'inherit',
                  fontWeight: 500,
                  opacity: !importPw || isDecrypting ? 0.5 : 1,
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  if (importPw && !isDecrypting) {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }
                }}
                onMouseLeave={(e) => {
                  if (importPw && !isDecrypting) {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-primary)';
                  }
                }}
              >
                {isDecrypting
                  ? t('common:loading', { defaultValue: '...' })
                  : t('settings:decrypt_and_preview')}
              </button>
            </div>
          )}

          {/* Decrypted preview — Page → Object → Attachment tree */}
          {decryptedPreview && (
            <>
              <div
                style={{
                  marginTop: 12,
                  borderTop: '1px solid var(--border-subtle)',
                  paddingTop: 12,
                }}
              >
                <h4 style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600, marginBottom: 6 }}>
                  {t('settings:select_objects')}
                </h4>

                {importPageGroups.length === 0 ? (
                  <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-tertiary)' }}>
                    {t('common:no_data')}
                  </p>
                ) : (
                  <div style={{ maxHeight: 320, overflowY: 'auto', fontSize: 'var(--text-body-sm)' }}>
                    {importPageGroups.map((group) => {
                      const allIds = group.objects.map((o) => o.id);
                      const pageChecked = importSelectedPageIds.has(group.sectionType);
                      const someChecked =
                        !pageChecked && allIds.some((id) => importSelections.get(id) === true);
                      const expanded = importExpandedPages.has(group.sectionType);
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
                            <SelectCheckbox
                              checked={pageChecked}
                              indeterminate={someChecked && !pageChecked}
                              onChange={() => onToggleImportPage(group.sectionType, allIds)}
                            />
                            <span
                              onClick={() => onToggleExpandedImportPage(group.sectionType)}
                              style={{
                                fontSize: 'var(--text-body)',
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
                                  fontSize: 'var(--text-badge)',
                                }}
                              >
                                ▶
                              </span>
                              <span
                                style={{
                                  overflow: 'hidden',
                                  textOverflow: 'ellipsis',
                                  whiteSpace: 'nowrap',
                                }}
                              >
                                {t(`navigation:${group.sectionType}`, group.sectionType)}
                              </span>
                            </span>
                            <span
                              style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}
                            >
                              {t('common:object_count', { n: group.objects.length })}
                            </span>
                          </div>

                          {/* Object rows (collapsible) */}
                          {expanded &&
                            group.objects.map((obj) => {
                              const isConflict = conflictIds.has(obj.id);
                              const isSelected = importSelections.get(obj.id) ?? true;
                              const objAtts = attachmentsByObject.get(obj.id) ?? [];
                              const hasAtts = objAtts.length > 0;
                              return (
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
                                    <SelectCheckbox
                                      checked={isSelected}
                                      onChange={() => onToggleSelection(obj.id)}
                                    />
                                    <span
                                      style={{
                                        fontSize: 'var(--text-body-sm)',
                                        flex: 1,
                                        overflow: 'hidden',
                                        textOverflow: 'ellipsis',
                                        whiteSpace: 'nowrap',
                                      }}
                                    >
                                      {obj.name}
                                    </span>
                                    <SensitivityBadge
                                      level={obj.sensitivityLevel as SensitivityLevel}
                                    />
                                    {isConflict && (
                                      <span
                                        style={{
                                          fontSize: 'var(--text-badge)',
                                          color: 'var(--warning)',
                                          border: '1px solid var(--warning)',
                                          borderRadius: 3,
                                          padding: '0 4px',
                                        }}
                                      >
                                        {t('settings:conflict')}
                                      </span>
                                    )}
                                    {hasAtts && (
                                      <button
                                        type="button"
                                        onClick={(e) => {
                                          e.preventDefault();
                                          e.stopPropagation();
                                          onToggleImportObjectExpanded(obj.id);
                                        }}
                                        style={{
                                          fontSize: 'var(--text-badge)',
                                          background: 'none',
                                          border: 'none',
                                          cursor: 'pointer',
                                          padding: '0 4px',
                                          transform: importExpandedObjects.has(obj.id)
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

                                  {/* Attachment rows */}
                                  {importExpandedObjects.has(obj.id) && (
                                    <div style={{ paddingLeft: 52, paddingBottom: 4 }}>
                                      {!hasAtts ? (
                                        <span
                                          style={{
                                            fontSize: 'var(--text-caption)',
                                            color: 'var(--text-tertiary)',
                                          }}
                                        >
                                          {t('settings:no_attachments')}
                                        </span>
                                      ) : (
                                        <>
                                          <div
                                            style={{
                                              display: 'flex',
                                              alignItems: 'center',
                                              gap: 4,
                                              padding: '2px 0',
                                              fontSize: 'var(--text-badge)',
                                              color: 'var(--text-tertiary)',
                                              borderBottom: '1px solid var(--border-subtle)',
                                              marginBottom: 2,
                                            }}
                                          >
                                            <Paperclip size={ICON_SIZE['2xs']} />
                                            <span>
                                              {t('settings:attachments_label')} ({objAtts.length})
                                            </span>
                                          </div>
                                          {objAtts.map((att) => (
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
                                              <SelectCheckbox
                                                checked={importSelectedAttachmentIds.has(att.id)}
                                                onChange={() => onToggleImportAttachment(att.id)}
                                              />
                                              <Paperclip
                                                size={ICON_SIZE['2xs']}
                                                style={{
                                                  color: 'var(--text-tertiary)',
                                                  flexShrink: 0,
                                                }}
                                              />
                                              <span style={{ fontSize: 'var(--text-caption)', flex: 1 }}>
                                                {att.fileName}
                                              </span>
                                              <span
                                                style={{
                                                  fontSize: 'var(--text-badge)',
                                                  color: 'var(--text-tertiary)',
                                                }}
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
                              );
                            })}
                        </div>
                      );
                    })}
                  </div>
                )}

                {decryptedPreview.conflicts.length > 0 && (
                  <div
                    style={{
                      marginTop: 8,
                      padding: '8px 12px',
                      background: 'var(--warning-subtle)',
                      borderRadius: 6,
                      fontSize: 'var(--text-caption)',
                      color: 'var(--warning)',
                    }}
                  >
                    {t('settings:conflict_warning', {
                      count: decryptedPreview.conflicts.length,
                    })}
                  </div>
                )}
              </div>

              {/* Action buttons */}
              {!showStrategySelector ? (
                <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
                  <button
                    type="button"
                    onClick={() => onSetShowStrategySelector(true)}
                    style={{
                      fontSize: 'var(--text-caption)',
                      padding: '6px 12px',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: 'var(--bg-toolbar)',
                      color: 'var(--text-primary)',
                      cursor: 'pointer',
                      fontFamily: 'inherit',
                      fontWeight: 500,
                      transition: 'all 0.15s ease',
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background =
                        'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                      e.currentTarget.style.borderColor = 'var(--accent-primary)';
                      e.currentTarget.style.color = 'var(--accent-primary)';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = 'var(--bg-toolbar)';
                      e.currentTarget.style.borderColor = 'var(--border-subtle)';
                      e.currentTarget.style.color = 'var(--text-primary)';
                    }}
                  >
                    {t('settings:advanced_import')}
                  </button>
                  <button
                    type="button"
                    onClick={onImport}
                    disabled={!importPw || isImporting || importTotalSelected === 0}
                    style={{
                      fontSize: 'var(--text-caption)',
                      padding: '6px 12px',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: isImporting
                        ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                        : 'var(--bg-toolbar)',
                      color: isImporting ? 'var(--accent-primary)' : 'var(--text-primary)',
                      cursor:
                        !importPw || isImporting || importTotalSelected === 0
                          ? 'default'
                          : 'pointer',
                      fontFamily: 'inherit',
                      fontWeight: 500,
                      opacity:
                        !importPw || isImporting || importTotalSelected === 0 ? 0.5 : 1,
                      transition: 'all 0.15s ease',
                    }}
                    onMouseEnter={(e) => {
                      if (importPw && !isImporting && importTotalSelected > 0) {
                        e.currentTarget.style.background =
                          'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                        e.currentTarget.style.borderColor = 'var(--accent-primary)';
                        e.currentTarget.style.color = 'var(--accent-primary)';
                      }
                    }}
                    onMouseLeave={(e) => {
                      if (importPw && !isImporting && importTotalSelected > 0) {
                        e.currentTarget.style.background = 'var(--bg-toolbar)';
                        e.currentTarget.style.borderColor = 'var(--border-subtle)';
                        e.currentTarget.style.color = 'var(--text-primary)';
                      }
                    }}
                  >
                    {isImporting
                      ? t('common:loading', { defaultValue: '...' })
                      : `${t('settings:quick_import')} (${importTotalSelected})`}
                  </button>
                </div>
              ) : (
                <div
                  style={{
                    marginTop: 12,
                    padding: 12,
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 8,
                  }}
                >
                  <h4 style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600, marginBottom: 8 }}>
                    {t('settings:import_strategy_title')}
                  </h4>
                  {(['skipExisting', 'overwrite', 'merge'] as ImportStrategy[]).map((s) => (
                    <label
                      key={s}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        padding: '6px 0',
                        cursor: 'pointer',
                        fontSize: 'var(--text-body-sm)',
                      }}
                    >
                      <input
                        type="radio"
                        checked={importStrategy === s}
                        onChange={() => onSetStrategy(s)}
                        style={{ accentColor: 'var(--accent-primary)' }}
                      />
                      <div>
                        <strong>{t(`settings:strategy_${s}`)}</strong>
                        <p
                          style={{
                            fontSize: 'var(--text-badge)',
                            color: 'var(--text-tertiary)',
                            margin: 1,
                          }}
                        >
                          {t(`settings:strategy_${s}_desc`)}
                        </p>
                      </div>
                    </label>
                  ))}
                  <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
                    <button
                      type="button"
                      onClick={() => onSetShowStrategySelector(false)}
                      style={{
                        fontSize: 'var(--text-caption)',
                        padding: '6px 12px',
                        borderRadius: 6,
                        border: '1px solid var(--border-subtle)',
                        background: 'var(--bg-toolbar)',
                        color: 'var(--text-primary)',
                        cursor: 'pointer',
                        fontFamily: 'inherit',
                        fontWeight: 500,
                        transition: 'all 0.15s ease',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background =
                          'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                        e.currentTarget.style.borderColor = 'var(--accent-primary)';
                        e.currentTarget.style.color = 'var(--accent-primary)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'var(--bg-toolbar)';
                        e.currentTarget.style.borderColor = 'var(--border-subtle)';
                        e.currentTarget.style.color = 'var(--text-primary)';
                      }}
                    >
                      {t('common:cancel')}
                    </button>
                    <button
                      type="button"
                      onClick={onImport}
                      disabled={!importPw || isImporting || importTotalSelected === 0}
                      style={{
                        fontSize: 'var(--text-caption)',
                        padding: '6px 12px',
                        borderRadius: 6,
                        border: '1px solid var(--border-subtle)',
                        background: isImporting
                          ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                          : 'var(--bg-toolbar)',
                        color: isImporting ? 'var(--accent-primary)' : 'var(--text-primary)',
                        cursor:
                          !importPw || isImporting || importTotalSelected === 0
                            ? 'default'
                            : 'pointer',
                        fontFamily: 'inherit',
                        fontWeight: 500,
                        opacity:
                          !importPw || isImporting || importTotalSelected === 0 ? 0.5 : 1,
                        transition: 'all 0.15s ease',
                      }}
                      onMouseEnter={(e) => {
                        if (importPw && !isImporting && importTotalSelected > 0) {
                          e.currentTarget.style.background =
                            'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                          e.currentTarget.style.borderColor = 'var(--accent-primary)';
                          e.currentTarget.style.color = 'var(--accent-primary)';
                        }
                      }}
                      onMouseLeave={(e) => {
                        if (importPw && !isImporting && importTotalSelected > 0) {
                          e.currentTarget.style.background = 'var(--bg-toolbar)';
                          e.currentTarget.style.borderColor = 'var(--border-subtle)';
                          e.currentTarget.style.color = 'var(--text-primary)';
                        }
                      }}
                    >
                      {isImporting
                        ? t('common:loading', { defaultValue: '...' })
                        : `${t('settings:import_action')} (${importTotalSelected})`}
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
        </Card>
      )}
      {importPreview && !decryptedPreview && (
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            textAlign: 'center',
          }}
        >
          {t('settings:password_required_for_decrypt')}
        </p>
      )}
    </>
  );
}
