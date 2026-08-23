import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { logger } from '@/lib/logger';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { saveWithPause } from '@/lib/dialog';
import { swapDocumentExt } from '@/lib/exportFormat';
import {
  cleanupStagedFile,
  copyStagedFileToDest,
  isUriPath,
  prepareStagedDownloadPath,
} from '@/lib/mobileFileTransfer';
import type { DocumentSensitivity, ExportDocumentResult, PageGroup } from '@/types/exportImport';

/** 导出格式。 */
export type DocFormat = 'docx' | 'pdf' | 'html' | 'txt' | 'markdown';

/** 各格式的扩展名与保存对话框过滤器。 */
const FORMAT_FILTERS: Record<DocFormat, { name: string; extensions: string[] }> = {
  docx: { name: 'Word Document', extensions: ['docx'] },
  pdf: { name: 'PDF Document', extensions: ['pdf'] },
  html: { name: 'HTML Document', extensions: ['html', 'htm'] },
  txt: { name: 'Plain Text', extensions: ['txt'] },
  markdown: { name: 'Markdown', extensions: ['md', 'markdown'] },
};

export interface UseExportDocumentSectionResult {
  selectedPageIds: Set<string>;
  expandedPages: Set<string>;
  totalSelected: number;
  isObjectSelected: (id: string) => boolean;
  togglePage: (sectionType: string, objectIds: string[]) => void;
  toggleObject: (objId: string) => void;
  toggleExpandedPage: (sectionType: string) => void;
  setSelectedPageIds: React.Dispatch<React.SetStateAction<Set<string>>>;
  setSelectedObjectIds: React.Dispatch<React.SetStateAction<Set<string>>>;
  format: DocFormat;
  setFormat: React.Dispatch<React.SetStateAction<DocFormat>>;
  savePath: string | null;
  isExporting: boolean;
  handleBrowse: () => Promise<void>;
  showWarning: boolean;
  setShowWarning: React.Dispatch<React.SetStateAction<boolean>>;
  showSensitiveConfirm: boolean;
  setShowSensitiveConfirm: React.Dispatch<React.SetStateAction<boolean>>;
  showPwDialog: boolean;
  setShowPwDialog: React.Dispatch<React.SetStateAction<boolean>>;
  pendingExportRef: React.MutableRefObject<(() => Promise<void>) | null>;
  handleWarningConfirmed: () => Promise<void>;
  handleSensitiveConfirmed: () => void;
  handleVerifyPassword: (password: string) => Promise<boolean>;
  handlePinSuccess: () => void;
  handleBiometricUnlock: () => Promise<boolean>;
  passwordHint: string | null;
  bioAvailable: { available: boolean; biometryType?: string };
}

/**
 * 「导出为文档」区块的状态与逻辑：勾选级联、格式/路径状态、三重确认
 * （明文警告 → 敏感度分级确认 → 审计日志）与全部解锁链路。
 * 与 ExportSection（.solosoul 加密导出）相互独立，各自维护勾选状态。
 */
export function useExportDocumentSection(
  accountId: string,
  pageGroups: PageGroup[],
): UseExportDocumentSectionResult {
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
  const prevFormatRef = useRef<DocFormat>(format);

  // 切换格式时，已选保存路径的扩展名跟随新格式更新（保留目录与文件名主体）。
  // 用 prevFormatRef 检测真实切换，避免在 StrictMode 双调用下重复改写。
  useEffect(() => {
    const prev = prevFormatRef.current;
    prevFormatRef.current = format;
    if (prev !== format && savePath) {
      setSavePath(swapDocumentExt(savePath, format));
    }
  }, [format, savePath]);

  // 确认流程状态
  const [showWarning, setShowWarning] = useState(false);
  const [showSensitiveConfirm, setShowSensitiveConfirm] = useState(false);
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pendingExportRef = useRef<(() => Promise<void>) | null>(null);

  // 当前账户密码提示词（critical 解密框提示按钮展示；与 ObjectDetailModal 同一数据源）
  const [passwordHint, setPasswordHint] = useState<string | null>(null);
  // 生物识别可用性（启用且已配置时才展示指纹/面容卡片，与关键数据查看框同一数据源）
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });

  useEffect(() => {
    let cancelled = false;
    invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
      .then((accounts) => {
        if (cancelled) return;
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      })
      .catch((err) => logger.warn('[ExportDocumentSection] Load password hint failed:', err));
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      { accountId },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch((err) =>
        logger.warn('[ExportDocumentSection] Biometric availability check failed:', err),
      );
    return () => {
      cancelled = true;
    };
  }, [accountId]);

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

  /** 验证成功后执行待导出的导出任务（三种解锁方式共用）。 */
  const executePendingExport = useCallback(async (): Promise<void> => {
    const fn = pendingExportRef.current;
    pendingExportRef.current = null;
    if (fn) await fn().catch(() => {}); // runExport 内部统一 toast
  }, []);

  /** 主密码验证（critical 分支的第二重确认）。 */
  const handleVerifyPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        await invoke('unlock_with_password', { accountId, password });
        // 验证成功后执行导出；true 让对话框关闭。
        await executePendingExport();
        return true;
      } catch (e) {
        // 与 ObjectDetailModal 一致：仅密码错误返回 false（对话框显示「密码不正确」），
        // 真实后端异常抛出，由对话框 catch 走 onError toast 保留细节。
        const msg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
        if (/invalid password|incorrect password|密码错误|密码不正确/i.test(msg)) {
          return false;
        }
        logger.warn('[ExportDocumentSection] Vault unlock failed:', e);
        throw e;
      }
    },
    [accountId, executePendingExport],
  );

  /** 生物识别解锁（与关键数据查看框同一链路，location 区分导出场景）。 */
  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', {
        accountId,
        location: 'document_export',
        action: 'unlock',
        biometryType: bioAvailable.biometryType,
      });
      await executePendingExport();
      return true;
    } catch (err) {
      logger.warn('[ExportDocumentSection] Biometric unlock failed:', err);
      return false;
    }
  }, [accountId, bioAvailable.biometryType, executePendingExport]);

  /** PIN 解锁成功（对话框内部负责 pin_unlock 调用）。 */
  const handlePinSuccess = useCallback(() => {
    void executePendingExport();
    setShowPwDialog(false);
  }, [executePendingExport]);

  const handleSensitiveConfirmed = useCallback(() => {
    setShowSensitiveConfirm(false);
    void runExport();
  }, [runExport]);

  const handleBrowse = useCallback(async () => {
    const fp = await saveWithPause({
      filters: [FORMAT_FILTERS[format]],
      // 用格式的主扩展名而非格式名（markdown → md），避免 .markdown.md 双后缀
      defaultPath: `SoloSoul_导出_${Date.now()}.${FORMAT_FILTERS[format].extensions[0]}`,
    });
    if (fp) setSavePath(fp);
  }, [format]);

  return {
    selectedPageIds,
    expandedPages,
    totalSelected,
    isObjectSelected: (id: string) => selectedObjectIds.has(id),
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
  };
}
