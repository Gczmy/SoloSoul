import { motion } from 'framer-motion';

import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { ICON_SIZE } from '@/lib/constants';
import { Info } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { ExportSection } from '@/components/export/ExportSection';
import { ExportDocumentSection } from '@/components/export/ExportDocumentSection';
import { ImportSection } from '@/components/import/ImportSection';
import { ExportImportTabBar } from '@/components/settings/ExportImportTabBar';

import { useExportImportPage } from './useExportImportPage';

/**
 * 导出/导入页 — P046 拆分后为纯展示组合层：
 * 全部编排逻辑（导出范围/密码/路径/估算、导入流程、Android URI 中转导出执行）
 * 收敛于 useExportImportPage 数据 hook；本组件仅负责 JSX 组合与子组件装配。
 */
export function ExportImportPage() {
  const {
    t,
    navigate,
    accountId,
    exportImportGuidePages,
    // tab
    tab,
    setTab,
    // scope
    scopeLoaded,
    scopeError,
    reloadScope,
    pageGroups,
    // export selection (useExportScope)
    selectedPageIds,
    selectedObjectIds,
    expandedPages,
    selectedAttachmentIds,
    objectAttachments,
    expandedObjects,
    totalSelected,
    togglePage,
    toggleObject,
    toggleObjectExpanded,
    toggleAttachment,
    toggleExpandedPage,
    bulkSelect,
    // export form state
    exportPassword,
    setExportPassword,
    exportPasswordConfirm,
    setExportPasswordConfirm,
    exportHint,
    setExportHint,
    savePath,
    setSavePath,
    cloudTargets,
    isExporting,
    showHintWarning,
    setShowHintWarning,
    showWeakPasswordWarning,
    setShowWeakPasswordWarning,
    selectedTags,
    setSelectedTags,
    includeAttachments,
    setIncludeAttachments,
    includePreferences,
    setIncludePreferences,
    includeBehavioral,
    setIncludeBehavioral,
    exportEstimate,
    estimating,
    hasSensitiveData,
    allTags,
    handleExport,
    // import (useImportState)
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
    objectConflictStrategies,
    importTotalSelected,
    handleSetImportPath,
    setImportPreview,
    setDecryptedPreview,
    setImportPw,
    setShowStrategySelector,
    setImportStrategy,
    handlePreviewImport,
    handleDecryptPreview,
    handleImport,
    toggleImportSelection,
    toggleImportPage,
    toggleImportAttachment,
    toggleExpandedImportPage,
    toggleImportObjectExpanded,
    handleSelectAllImport,
    handleSetObjectConflictStrategy,
    // warning-skip refs
    skipHintCheckRef,
    skipWeakPasswordCheckRef,
  } = useExportImportPage();

  return (
    <PageShell
      title={t('settings:export_import')}
      onBack={() => navigate('/settings')}
      actions={<PageGuideButton pages={exportImportGuidePages} />}
    >
      <PageContainer variant="medium" gap="default">

        <ExportImportTabBar tab={tab} onChange={setTab} />

        {(tab === 'export' || tab === 'document') && scopeLoaded && scopeError ? (
          // N-11: 加载失败态与「空数据」同态问题——失败时显示错误占位 + 重试，
          // 不再渲染空导出树（用户误以为数据丢失）。
          <Card style={{ padding: '48px 24px', textAlign: 'center' }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 12 }}>
              <Info size={ICON_SIZE['2xl']} style={{ opacity: 0.4, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
                {t('settings:export_scope_load_failed', { defaultValue: '导出范围加载失败，请重试' })}
              </p>
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  maxWidth: 420,
                  wordBreak: 'break-word',
                }}
              >
                {scopeError}
              </p>
              <Button variant="primary" size="sm" onClick={reloadScope}>
                {t('common:retry')}
              </Button>
            </div>
          </Card>
        ) : tab === 'export' && scopeLoaded ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            style={{ display: 'flex', flexDirection: 'column', gap: 'var(--page-gap)' }}
          >
            <ExportSection
              pageGroups={pageGroups}
              selectedPageIds={selectedPageIds}
              selectedObjectIds={selectedObjectIds}
              expandedPages={expandedPages}
              exportPassword={exportPassword}
              exportPasswordConfirm={exportPasswordConfirm}
              exportHint={exportHint}
              savePath={savePath}
              cloudTargets={cloudTargets}
              isExporting={isExporting}
              showHintWarning={showHintWarning}
              selectedTags={selectedTags}
              includeAttachments={includeAttachments}
              selectedAttachmentIds={selectedAttachmentIds}
              objectAttachments={objectAttachments}
              expandedObjects={expandedObjects}
              includePreferences={includePreferences}
              includeBehavioral={includeBehavioral}
              exportEstimate={exportEstimate}
              estimating={estimating}
              hasSensitiveData={hasSensitiveData}
              allTags={allTags}
              totalSelected={totalSelected}
              onTogglePage={togglePage}
              onToggleObject={toggleObject}
              onToggleObjectExpanded={toggleObjectExpanded}
              onToggleAttachment={toggleAttachment}
              onSetExportPassword={setExportPassword}
              onSetExportPasswordConfirm={setExportPasswordConfirm}
              onSetExportHint={setExportHint}
              onSetSavePath={setSavePath}
              onExport={handleExport}
              onSetShowHintWarning={setShowHintWarning}
              onSetSelectedTags={(updater) => setSelectedTags(updater)}
              onSetIncludeAttachments={setIncludeAttachments}
              onSetIncludePreferences={setIncludePreferences}
              onSetIncludeBehavioral={setIncludeBehavioral}
              onToggleExpandedPage={toggleExpandedPage}
              onSelectAllExport={(selectAll) =>
                bulkSelect(
                  selectAll,
                  pageGroups.flatMap((g) => g.objects.map((o) => o.id)),
                  pageGroups.map((g) => g.sectionType),
                )
              }
              showWeakPasswordWarning={showWeakPasswordWarning}
              onSetShowWeakPasswordWarning={setShowWeakPasswordWarning}
              onSetShowHintWarningAndExport={() => {
                skipHintCheckRef.current = true;
                setShowHintWarning(false);
                handleExport();
              }}
              onSetWeakPasswordExport={() => {
                skipWeakPasswordCheckRef.current = true;
                setShowWeakPasswordWarning(false);
                handleExport();
              }}
            />
          </motion.div>
        ) : tab === 'document' && scopeLoaded ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            style={{ display: 'flex', flexDirection: 'column', gap: 'var(--page-gap)' }}
          >
            <ExportDocumentSection accountId={accountId} pageGroups={pageGroups} />
          </motion.div>
        ) : tab === 'import' ? (
          <ImportSection
            importPath={importPath}
            importPreview={importPreview}
            importPw={importPw}
            decryptedPreview={decryptedPreview}
            isPreviewing={isPreviewing}
            isDecrypting={isDecrypting}
            isImporting={isImporting}
            importStrategy={importStrategy}
            importSelections={importSelections}
            showStrategySelector={showStrategySelector}
            importSelectedPageIds={importSelectedPageIds}
            importSelectedAttachmentIds={importSelectedAttachmentIds}
            importExpandedPages={importExpandedPages}
            importExpandedObjects={importExpandedObjects}
            importTotalSelected={importTotalSelected}
            onSetImportPath={handleSetImportPath}
            onSetImportPreview={setImportPreview}
            onSetDecryptedPreview={setDecryptedPreview}
            onSetImportPw={setImportPw}
            onSetShowStrategySelector={setShowStrategySelector}
            onPreview={handlePreviewImport}
            onDecrypt={handleDecryptPreview}
            onImport={handleImport}
            onToggleSelection={toggleImportSelection}
            onToggleImportPage={toggleImportPage}
            onToggleImportAttachment={toggleImportAttachment}
            onToggleExpandedImportPage={toggleExpandedImportPage}
            onToggleImportObjectExpanded={toggleImportObjectExpanded}
            onSelectAllImport={handleSelectAllImport}
            onSetStrategy={setImportStrategy}
            objectConflictStrategies={objectConflictStrategies}
            onSetObjectConflictStrategy={handleSetObjectConflictStrategy}
          />
        ) : null}
      </PageContainer>
    </PageShell>
  );
}
