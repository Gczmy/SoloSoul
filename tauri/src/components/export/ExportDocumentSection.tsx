import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FileText, AlertTriangle } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { TransferButton } from '@/components/transfer/TransferButton';
import { ObjectSelectionTree } from '@/components/transfer/ObjectSelectionTree';
import { ConfirmDialog } from '@/components/attachment/ConfirmDialog';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { logger } from '@/lib/logger';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { saveWithPause } from '@/lib/dialog';
import {
  cleanupStagedFile,
  copyStagedFileToDest,
  isUriPath,
  prepareStagedDownloadPath,
} from '@/lib/mobileFileTransfer';
import { ICON_SIZE } from '@/lib/constants';
import type {
  DocumentSensitivity,
  ExportDocumentResult,
  PageGroup,
} from '@/types/exportImport';

interface ExportDocumentSectionProps {
  accountId: string;
  pageGroups: PageGroup[];
}

/** 导出格式。 */
type DocFormat = 'docx' | 'pdf' | 'html';

/** 各格式的扩展名与保存对话框过滤器。 */
const FORMAT_FILTERS: Record<DocFormat, { name: string; extensions: string[] }> = {
  docx: { name: 'Word Document', extensions: ['docx'] },
  pdf: { name: 'PDF Document', extensions: ['pdf'] },
  html: { name: 'HTML Document', extensions: ['html', 'htm'] },
};

/**
 * 「导出为文档」区块：复用 ObjectSelectionTree 勾选对象 → 格式选择器 → 保存路径 →
 * 三重确认（明文警告 → 敏感度分级确认 → 审计日志）→ export_objects_document。
 *
 * 与 ExportSection（.solosoul 加密导出）相互独立，各自维护勾选状态。
 */
export function ExportDocumentSection({ accountId, pageGroups }: ExportDocumentSectionProps) {
  const { t } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();

  // 勾选状态（页面级全选级联到对象）
  const [selectedPageIds, setSelectedPageIds] = useState<Set<string>>(new Set());
  const [selectedObjectIds, setSelectedObjectIds] = useState<Set<string>>(new Set());
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());

  // 导出状态
  const [format, setFormat] = useState<DocFormat>('docx');
  const [savePath, setSavePath] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);

  // 确认流程状态
  const [showWarning, setShowWarning] = useState(false);
  const [showSensitiveConfirm, setShowSensitiveConfirm] = useState(false);
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pendingExportRef = useRef<(() => Promise<void>) | null>(null);

  // 选中的对象 id 列表——按勾选树 页面→对象 展示顺序排列（Rust 侧按此顺序分页）
  const orderedObjectIds = useMemo(() => {
    const ids: string[] = [];
    for (const group of pageGroups) {
      for (const obj of group.objects) {
        if (selectedObjectIds.has(obj.id)) ids.push(obj.id);
      }
    }
    return ids;
  }, [pageGroups, selectedObjectIds]);

  const totalSelected = selectedObjectIds.size;

  // 注意：不在 setState updater 内部调用另一个 setState（StrictMode 双调用会导致
  // 级联状态被重复切换）。页面/对象两组集合一起变——基于当前闭包快照独立计算后再
  // 分别 setState（callback 依赖两组状态，选择变化时自动重建，无过期闭包问题）。
  const togglePage = useCallback(
    (sectionType: string, objectIds: string[]) => {
      const willSelect = !selectedPageIds.has(sectionType);
      const nextPages = new Set(selectedPageIds);
      const nextObjects = new Set(selectedObjectIds);
      if (willSelect) {
        nextPages.add(sectionType);
        for (const id of objectIds) nextObjects.add(id);
      } else {
        nextPages.delete(sectionType);
        for (const id of objectIds) nextObjects.delete(id);
      }
      setSelectedPageIds(nextPages);
      setSelectedObjectIds(nextObjects);
    },
    [selectedPageIds, selectedObjectIds],
  );

  const toggleObject = useCallback((objId: string) => {
    setSelectedObjectIds((prev) => {
      const next = new Set(prev);
      if (next.has(objId)) next.delete(objId);
      else next.add(objId);
      return next;
    });
  }, []);

  const toggleExpandedPage = useCallback((sectionType: string) => {
    setExpandedPages((prev) => {
      const next = new Set(prev);
      if (next.has(sectionType)) next.delete(sectionType);
      else next.add(sectionType);
      return next;
    });
  }, []);

  /** 核心导出动作（第三重确认后执行）。 */
  const runExport = useCallback(async (): Promise<void> => {
    if (!savePath || orderedObjectIds.length === 0) return;
    setIsExporting(true);
    let stagedPath: string | null = null;
    try {
      let targetSavePath = savePath;
      if (isUriPath(savePath)) {
        stagedPath = await prepareStagedDownloadPath(`SoloSoul_导出.${format}`);
        targetSavePath = stagedPath;
      }
      const result = await invoke<ExportDocumentResult>('export_objects_document', {
        objectIds: orderedObjectIds,
        savePath: targetSavePath,
        format,
      });
      if (stagedPath) {
        await copyStagedFileToDest(stagedPath, savePath);
      }
      onSuccess(
        t('settings:export_doc_success', {
          count: result.objectCount,
          defaultValue: `Document exported successfully (${result.objectCount} objects)`,
        }),
      );
      // 导出成功后清空勾选与路径（防止重复导出旧内容）
      setSelectedPageIds(new Set());
      setSelectedObjectIds(new Set());
      setSavePath(null);
    } catch (e) {
      logger.warn('[ExportDocumentSection] Export failed:', e);
      onError(new Error(resolveBackendErrorMessage(e)), t('common:export_failed'));
    } finally {
      if (stagedPath) await cleanupStagedFile(stagedPath).catch(() => {});
      setIsExporting(false);
    }
  }, [savePath, orderedObjectIds, format, onError, onSuccess, t]);

  /** 第一重确认通过后：preflight 分级 → 第二重（critical 密码框 / sensitive 二次确认 / none 直接导出）。 */
  const handleWarningConfirmed = useCallback(async () => {
    setShowWarning(false);
    try {
      const maxSensitivity = await invoke<DocumentSensitivity>('export_document_preflight', {
        objectIds: orderedObjectIds,
      });
      if (maxSensitivity === 'critical') {
        pendingExportRef.current = runExport;
        setShowPwDialog(true);
      } else if (maxSensitivity === 'sensitive') {
        setShowSensitiveConfirm(true);
      } else {
        await runExport();
      }
    } catch (e) {
      logger.warn('[ExportDocumentSection] Preflight failed:', e);
      onError(new Error(resolveBackendErrorMessage(e)), t('settings:export_doc_preflight_failed'));
    }
  }, [orderedObjectIds, runExport, onError, t]);

  /** 主密码验证（critical 分支的第二重确认）。 */
  const handleVerifyPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        await invoke('unlock_with_password', { accountId, password });
        // 验证成功后执行导出；runExport 失败时 Promise 以 boolean 形式抛出——此处吞掉，
        // 由 runExport 内部统一 toast。true 让对话框关闭。
        const fn = pendingExportRef.current;
        pendingExportRef.current = null;
        if (fn) await fn().catch(() => {});
        return true;
      } catch {
        return false; // 密码错误 → 对话框显示「密码不正确」
      }
    },
    [accountId],
  );

  const handleSensitiveConfirmed = useCallback(() => {
    setShowSensitiveConfirm(false);
    void runExport();
  }, [runExport]);

  const handleBrowse = useCallback(async () => {
    const fp = await saveWithPause({
      filters: [FORMAT_FILTERS[format]],
      defaultPath: `SoloSoul_导出_${Date.now()}.${format}`,
    });
    if (fp) setSavePath(fp);
  }, [format]);

  return (
    <>
      <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
        {t('settings:export_doc_desc', {
          defaultValue: 'Export selected objects as a readable document. Each object becomes one page.',
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
          isObjectSelected={(id) => selectedObjectIds.has(id)}
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
          {(['docx', 'pdf', 'html'] as DocFormat[]).map((f) => {
            const active = format === f;
            const label =
              f === 'docx'
                ? t('settings:export_format_word')
                : f === 'pdf'
                  ? t('settings:export_format_pdf')
                  : t('settings:export_format_html');
            return (
              <button
                key={f}
                type="button"
                onClick={() => setFormat(f)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
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
                <FileText size={ICON_SIZE.sm} />
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

      {/* 第二重（critical）：主密码验证框 */}
      <PasswordVerificationDialog
        open={showPwDialog}
        onClose={() => {
          pendingExportRef.current = null;
          setShowPwDialog(false);
        }}
        onVerify={handleVerifyPassword}
        title={t('common:export_doc_critical_title', { defaultValue: 'Verify master password' })}
        description={t('common:export_doc_critical_desc', {
          defaultValue: 'Selected objects contain critical fields. Verify your master password to export.',
        })}
        confirmLabel={t('common:continue')}
      />
    </>
  );
}
