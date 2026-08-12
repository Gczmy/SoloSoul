import { useMemo } from 'react';
import type { TFunction } from 'i18next';
import { ObjectSelectionTree } from '@/components/transfer/ObjectSelectionTree';
import type {
  AttachmentImportInfo,
  ConflictKind,
  DecryptedImportPreview,
  ExportObjectSummary,
  ImportStrategy,
} from '@/types/exportImport';

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

/**
 * ImportSection 的解密预览区（P046 拆分：展示子组件）。
 * 页面→对象→附件选择树 + 冲突徽章 + 逐对象冲突策略选择器。
 */
export function ImportDecryptedSection({
  decryptedPreview,
  importSelections,
  importSelectedPageIds,
  importSelectedAttachmentIds,
  importExpandedPages,
  importExpandedObjects,
  importTotalSelected,
  importStrategy,
  objectConflictStrategies,
  onToggleSelection,
  onToggleImportPage,
  onToggleImportAttachment,
  onToggleExpandedImportPage,
  onToggleImportObjectExpanded,
  onSelectAllImport,
  onSetObjectConflictStrategy,
  t,
}: {
  decryptedPreview: DecryptedImportPreview;
  importSelections: Map<string, boolean>;
  importSelectedPageIds: Set<string>;
  importSelectedAttachmentIds: Set<string>;
  importExpandedPages: Set<string>;
  importExpandedObjects: Set<string>;
  importTotalSelected: number;
  importStrategy: ImportStrategy;
  objectConflictStrategies: Map<string, ImportStrategy>;
  onToggleSelection: (id: string) => void;
  onToggleImportPage: (sectionType: string, objectIds: string[]) => void;
  onToggleImportAttachment: (attId: string) => void;
  onToggleExpandedImportPage: (sectionType: string) => void;
  onToggleImportObjectExpanded: (objectId: string) => void;
  onSelectAllImport: (selectAll: boolean) => void;
  onSetObjectConflictStrategy: (objectId: string, strategy: ImportStrategy) => void;
  t: TFunction;
}) {
  const importPageGroups = useMemo(
    () => groupIntoPages(decryptedPreview.objects),
    [decryptedPreview],
  );

  const conflictIds = useMemo(
    () => new Set(decryptedPreview.conflicts.map((c) => c.objectId)),
    [decryptedPreview],
  );

  const conflictMap = useMemo(
    () => new Map(decryptedPreview.conflicts.map((c) => [c.objectId, c])),
    [decryptedPreview],
  );

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
    for (const att of decryptedPreview.attachments) {
      let list = map.get(att.objectId);
      if (!list) {
        list = [];
        map.set(att.objectId, list);
      }
      list.push(att);
    }
    return map;
  }, [decryptedPreview]);

  return (
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
    </>
  );
}
