import { useTranslation } from 'react-i18next';
import { AlertTriangle } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { TransferButton } from '@/components/transfer/TransferButton';
import { ObjectSelectionTree } from '@/components/transfer/ObjectSelectionTree';
import { ConfirmDialog } from '@/components/attachment/ConfirmDialog';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ICON_SIZE } from '@/lib/constants';
import type { PageGroup } from '@/types/exportImport';
import { useExportDocumentSection, type DocFormat } from './useExportDocumentSection';

interface ExportDocumentSectionProps {
  accountId: string;
  pageGroups: PageGroup[];
}

/**
 * 「导出为文档」区块：复用 ObjectSelectionTree 勾选对象 → 格式选择器 → 保存路径 →
 * 三重确认（明文警告 → 敏感度分级确认 → 审计日志）→ export_objects_document。
 *
 * 与 ExportSection（.solosoul 加密导出）相互独立，各自维护勾选状态。
 */
export function ExportDocumentSection({ accountId, pageGroups }: ExportDocumentSectionProps) {
  const { t } = useTranslation(['settings', 'common']);
  const {
    selectedPageIds,
    expandedPages,
    totalSelected,
    isObjectSelected,
    togglePage,
    toggleObject,
    toggleExpandedPage,
    setSelectedPageIds,
    setSelectedObjectIds,
    format,
    setFormat,
    savePath,
    isExporting,
    handleBrowse,
    showWarning,
    setShowWarning,
    showSensitiveConfirm,
    setShowSensitiveConfirm,
    showPwDialog,
    setShowPwDialog,
    pendingExportRef,
    handleWarningConfirmed,
    handleSensitiveConfirmed,
    handleVerifyPassword,
    handlePinSuccess,
    handleBiometricUnlock,
    passwordHint,
    bioAvailable,
  } = useExportDocumentSection(accountId, pageGroups);

  return (
    <>
      <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
        {t('settings:export_doc_desc', {
          defaultValue:
            'Export selected objects as a readable document. Each object becomes one page.',
        })}
      </p>

      {/* 对象选择树 */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('settings:select_objects')}
        </h3>
        <ObjectSelectionTree
          pageGroups={pageGroups}
          selectedPageIds={selectedPageIds}
          expandedPages={expandedPages}
          expandedObjects={new Set()}
          selectedAttachmentIds={new Set()}
          objectAttachments={new Map()}
          totalSelected={totalSelected}
          showAttachmentExpand={() => false}
          isObjectSelected={isObjectSelected}
          onTogglePage={togglePage}
          onToggleObject={(objId) => toggleObject(objId)}
          onToggleObjectExpanded={() => {}}
          onToggleAttachment={() => {}}
          onToggleExpandedPage={toggleExpandedPage}
          onSelectAll={(selectAll) => {
            const allIds = pageGroups.flatMap((g) => g.objects.map((o) => o.id));
            if (selectAll) {
              setSelectedPageIds(new Set(pageGroups.map((g) => g.sectionType)));
              setSelectedObjectIds(new Set(allIds));
            } else {
              setSelectedPageIds(new Set());
              setSelectedObjectIds(new Set());
            }
          }}
        />
      </Card>

      {/* 格式选择器 */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('settings:export_format_label', { defaultValue: 'Format' })}
        </h3>
        <div style={{ display: 'flex', gap: 8 }}>
          {(['docx', 'pdf', 'html', 'txt', 'markdown'] as DocFormat[]).map((f) => {
            const active = format === f;
            const label =
              f === 'docx'
                ? t('settings:export_format_word')
                : f === 'pdf'
                  ? t('settings:export_format_pdf')
                  : f === 'html'
                    ? t('settings:export_format_html')
                    : f === 'txt'
                      ? t('settings:export_format_txt')
                      : t('settings:export_format_markdown');
            return (
              <button
                key={f}
                type="button"
                onClick={() => setFormat(f)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  padding: '8px 14px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: active ? 'var(--accent-primary)' : 'var(--bg-elevated)',
                  color: active ? '#fff' : 'var(--text-primary)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  fontSize: 'var(--text-body-sm)',
                }}
              >
                {label}
              </button>
            );
          })}
        </div>
      </Card>

      {/* 保存路径 */}
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
          {t('common:export_path')}
        </h3>
        <div
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 8,
            wordBreak: 'break-all',
          }}
        >
          {savePath || t('settings:no_file_selected')}
        </div>
        <TransferButton onClick={handleBrowse}>{t('common:browse')}</TransferButton>
      </Card>

      {/* 导出按钮 */}
      <TransferButton
        variant="accent"
        onClick={() => setShowWarning(true)}
        disabled={totalSelected === 0 || !savePath}
        busy={isExporting}
      >
        {isExporting
          ? t('common:loading', { defaultValue: '...' })
          : `${t('settings:export_doc_button', { defaultValue: 'Export as document' })} (${totalSelected})`}
      </TransferButton>

      {/* 第一重：明文导出警告 */}
      <ConfirmDialog
        open={showWarning}
        title={t('common:export_doc_warning_title')}
        body={
          <div style={{ display: 'flex', gap: 10, alignItems: 'flex-start' }}>
            <AlertTriangle size={ICON_SIZE.lg} style={{ color: 'var(--warning)', flexShrink: 0 }} />
            <span>{t('common:export_doc_warning_body')}</span>
          </div>
        }
        confirmLabel={t('common:continue')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={handleWarningConfirmed}
        onCancel={() => setShowWarning(false)}
      />

      {/* 第二重（sensitive）：二次确认 */}
      <ConfirmDialog
        open={showSensitiveConfirm}
        title={t('common:export_doc_sensitive_confirm_title', {
          defaultValue: 'Sensitive fields',
        })}
        body={t('common:export_doc_sensitive_confirm')}
        confirmLabel={t('common:continue')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={handleSensitiveConfirmed}
        onCancel={() => setShowSensitiveConfirm(false)}
      />

      {/* 第二重（critical）：验证框——复用关键数据查看的统一验证框
          （支持指纹/面容/PIN/主密码多解锁方式，文案按导出场景定制） */}
      <PasswordVerificationDialog
        open={showPwDialog}
        onClose={() => {
          pendingExportRef.current = null;
          setShowPwDialog(false);
        }}
        onVerify={handleVerifyPassword}
        title={t('common:export_doc_critical_title', { defaultValue: 'Verify master password' })}
        description={t('common:export_doc_critical_desc', {
          defaultValue:
            'Selected objects contain critical fields. Verify your master password to export.',
        })}
        confirmLabel={t('common:unlock')}
        hint={passwordHint}
        pinAccountId={accountId}
        onPinSuccess={handlePinSuccess}
        biometricType={bioAvailable.available ? bioAvailable.biometryType : undefined}
        onBiometric={bioAvailable.available ? handleBiometricUnlock : undefined}
      />
    </>
  );
}
