import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import type {
  ImportStrategy,
  ImportPreview,
  DecryptedImportPreview,
} from '@/types/exportImport';

import { ImportFileSelectorCard } from './ImportFileSelectorCard';
import { ImportManifestInfoSection } from './ImportManifestInfoSection';
import { ImportDecryptedSection } from './ImportDecryptedSection';
import { ImportActionSection } from './ImportActionSection';

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

/**
 * 导入配置区 — P046 拆分后为纯组合层：
 * 文件选择（ImportFileSelectorCard）、清单解密（ImportManifestInfoSection）、
 * 解密预览树+冲突（ImportDecryptedSection）、操作/策略（ImportActionSection）
 * 均为独立展示子组件；本组件保留卡片外壳与条件渲染编排。
 */
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

  return (
    <>
      <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
        {t('settings:import_desc')}
      </p>

      {/* File selector（P046 拆分：ImportFileSelectorCard） */}
      <ImportFileSelectorCard
        importPath={importPath}
        importPreview={importPreview}
        isPreviewing={isPreviewing}
        onSetImportPath={onSetImportPath}
        onSetImportPreview={onSetImportPreview}
        onSetDecryptedPreview={onSetDecryptedPreview}
        onSetImportPw={onSetImportPw}
        onSetShowStrategySelector={onSetShowStrategySelector}
        onPreview={onPreview}
        t={t}
      />

      {/* Parsed manifest preview */}
      {importPreview && (
        <Card>
          <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
            {t('settings:import_preview')}
          </h3>

          {/* 清单信息 + 密码解密（P046 拆分：ImportManifestInfoSection） */}
          <ImportManifestInfoSection
            importPreview={importPreview}
            importPw={importPw}
            isDecrypting={isDecrypting}
            decryptedPreview={decryptedPreview}
            onSetImportPw={onSetImportPw}
            onDecrypt={onDecrypt}
            t={t}
          />

          {/* Decrypted preview — Page → Object → Attachment tree */}
          {decryptedPreview && (
            <>
              {' '}
              {/* 解密预览树 + 冲突（P046 拆分：ImportDecryptedSection） */}
              <ImportDecryptedSection
                decryptedPreview={decryptedPreview}
                importSelections={importSelections}
                importSelectedPageIds={importSelectedPageIds}
                importSelectedAttachmentIds={importSelectedAttachmentIds}
                importExpandedPages={importExpandedPages}
                importExpandedObjects={importExpandedObjects}
                importTotalSelected={importTotalSelected}
                importStrategy={importStrategy}
                objectConflictStrategies={objectConflictStrategies}
                onToggleSelection={onToggleSelection}
                onToggleImportPage={onToggleImportPage}
                onToggleImportAttachment={onToggleImportAttachment}
                onToggleExpandedImportPage={onToggleExpandedImportPage}
                onToggleImportObjectExpanded={onToggleImportObjectExpanded}
                onSelectAllImport={onSelectAllImport}
                onSetObjectConflictStrategy={onSetObjectConflictStrategy}
                t={t}
              />
              {/* 操作区（P046 拆分：ImportActionSection） */}
              <ImportActionSection
                showStrategySelector={showStrategySelector}
                importStrategy={importStrategy}
                isImporting={isImporting}
                importPw={importPw}
                importTotalSelected={importTotalSelected}
                onSetShowStrategySelector={onSetShowStrategySelector}
                onSetStrategy={onSetStrategy}
                onImport={onImport}
                t={t}
              />
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
