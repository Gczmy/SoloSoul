import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { ObjectSelectionTree } from '@/components/transfer/ObjectSelectionTree';
import { TransferButton } from '@/components/transfer/TransferButton';
import type {
  AttachmentImportInfo,
  ImportStrategy,
  ImportPreview,
  DecryptedImportPreview,
  ExportObjectSummary,
  ConflictKind,
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
  onSelectAllImport: (selectAll: boolean) => void;
  onSetStrategy: (s: ImportStrategy) => void;
  // Per-object strategy for conflicts
  objectConflictStrategies: Map<string, ImportStrategy>;
  onSetObjectConflictStrategy: (objectId: string, strategy: ImportStrategy) => void;
}

/** Group decrypted preview objects by section_type into pages */
function groupIntoPages(objects: ExportObjectSummary[]) {
  const map = new Map<string, { sectionType: string; objects: ExportObjectSummary[] }>();
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
  onSelectAllImport,
  onSetStrategy,
  objectConflictStrategies,
  onSetObjectConflictStrategy,
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

  // Build a conflict lookup map for quick access
  const conflictMap = useMemo(
    () => new Map(decryptedPreview?.conflicts.map((c) => [c.objectId, c]) ?? []),
    [decryptedPreview],
  );

  // Text for conflict kind
  const conflictKindText = (kind: ConflictKind): string => {
    switch (kind) {
      case 'identical':
        return t('settings:conflict_kind_identical');
      case 'renamedLocal':
        return t('settings:conflict_kind_renamed_local');
    }
  };

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
            // Android content:// URI 很长，折行防止溢出卡片
            wordBreak: 'break-all',
          }}
        >
          {importPath || t('settings:no_file_selected')}
        </div>
        <TransferButton
          onClick={async () => {
            const { openWithPause } = await import('@/lib/dialog');
            const selected = await openWithPause({
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
        >
          {t('settings:select_file')}
        </TransferButton>
        {importPath && !importPreview && (
          <div style={{ marginTop: 8 }}>
            <TransferButton onClick={onPreview} disabled={isPreviewing} busy={isPreviewing}>
              {isPreviewing ? t('common:loading', { defaultValue: '...' }) : t('settings:preview')}
            </TransferButton>
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
              <TransferButton
                onClick={onDecrypt}
                disabled={!importPw || isDecrypting}
                busy={isDecrypting}
              >
                {isDecrypting
                  ? t('common:loading', { defaultValue: '...' })
                  : t('settings:decrypt_and_preview')}
              </TransferButton>
            </div>
          )}

          {/* Decrypted preview — Page → Object → Attachment tree */}
          {decryptedPreview && (
            <>
              {' '}
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
                <ObjectSelectionTree
                  pageGroups={importPageGroups}
                  selectedPageIds={importSelectedPageIds}
                  expandedPages={importExpandedPages}
                  expandedObjects={importExpandedObjects}
                  selectedAttachmentIds={importSelectedAttachmentIds}
                  objectAttachments={attachmentsByObject}
                  totalSelected={importTotalSelected}
                  showAttachmentExpand={(obj) => (attachmentsByObject.get(obj.id) ?? []).length > 0}
                  isObjectSelected={(id) => importSelections.get(id) ?? true}
                  onTogglePage={onToggleImportPage}
                  onToggleObject={(id) => onToggleSelection(id)}
                  onToggleObjectExpanded={onToggleImportObjectExpanded}
                  onToggleAttachment={(attId) => onToggleImportAttachment(attId)}
                  onToggleExpandedPage={onToggleExpandedImportPage}
                  onSelectAll={onSelectAllImport}
                  scrollable
                  renderConflictBadge={(obj) => {
                    if (!conflictIds.has(obj.id)) return null;
                    const cinfo = conflictMap.get(obj.id);
                    return (
                      <span
                        title={cinfo ? conflictKindText(cinfo.kind) : ''}
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
                    );
                  }}
                />

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
                    <div style={{ fontWeight: 600, marginBottom: 4 }}>
                      {t('settings:conflict_warning', {
                        count: decryptedPreview.conflicts.length,
                      })}
                    </div>
                    {/* Per-object conflict strategy selector */}
                    {decryptedPreview.conflicts.map((c) => {
                      const currentStrategy =
                        objectConflictStrategies.get(c.objectId) ?? importStrategy;
                      return (
                        <div
                          key={c.objectId}
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 8,
                            padding: '4px 0',
                            borderBottom: '1px solid var(--border-subtle)',
                            flexWrap: 'wrap',
                          }}
                        >
                          <span style={{ flex: 1, minWidth: 120, fontSize: 'var(--text-badge)' }}>
                            {c.importedName}
                            {c.importedName !== c.existingName && (
                              <span style={{ color: 'var(--text-tertiary)' }}>
                                {' '}
                                ← 本地: {c.existingName}
                              </span>
                            )}
                          </span>
                          <span
                            style={{
                              fontSize: 'var(--text-badge)',
                              color: 'var(--warning)',
                              padding: '0 4px',
                            }}
                          >
                            {conflictKindText(c.kind)}
                          </span>
                          <select
                            value={currentStrategy}
                            onChange={(e) =>
                              onSetObjectConflictStrategy(
                                c.objectId,
                                e.target.value as ImportStrategy,
                              )
                            }
                            style={{
                              fontSize: 'var(--text-caption)',
                              padding: '2px 6px',
                              borderRadius: 4,
                              border: '1px solid var(--border-subtle)',
                              background: 'var(--bg-toolbar)',
                              color: 'var(--text-primary)',
                              fontFamily: 'inherit',
                              cursor: 'pointer',
                            }}
                          >
                            <option value="skipExisting">
                              {t('settings:strategy_skipExisting')}
                            </option>
                            <option value="overwrite">{t('settings:strategy_overwrite')}</option>
                            <option value="keepBoth">{t('settings:strategy_keepBoth')}</option>
                          </select>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
              {/* Action buttons */}
              {!showStrategySelector ? (
                <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
                  <TransferButton onClick={() => onSetShowStrategySelector(true)}>
                    {t('settings:advanced_import')}
                  </TransferButton>
                  <TransferButton
                    variant="accent"
                    onClick={onImport}
                    disabled={!importPw || isImporting || importTotalSelected === 0}
                    busy={isImporting}
                  >
                    {isImporting
                      ? t('common:loading', { defaultValue: '...' })
                      : `${t('settings:quick_import')} (${importTotalSelected})`}
                  </TransferButton>
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
                  {(['skipExisting', 'overwrite', 'keepBoth'] as ImportStrategy[]).map((s) => (
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
                    <TransferButton onClick={() => onSetShowStrategySelector(false)}>
                      {t('common:cancel')}
                    </TransferButton>
                    <TransferButton
                      variant="accent"
                      onClick={onImport}
                      disabled={!importPw || isImporting || importTotalSelected === 0}
                      busy={isImporting}
                    >
                      {isImporting
                        ? t('common:loading', { defaultValue: '...' })
                        : `${t('settings:import_action')} (${importTotalSelected})`}
                    </TransferButton>
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
