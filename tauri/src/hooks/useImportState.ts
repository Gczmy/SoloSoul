import { useState, useCallback, useMemo } from 'react';
import type { TFunction, i18n as I18n } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { cleanupStagedFile, isUriPath, stageImportPackage } from '@/lib/mobileFileTransfer';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import type {
  ImportPreview,
  DecryptedImportPreview,
  ImportStrategy,
  ImportResult,
} from '@/types/exportImport';

/**
 * P013/3: 导入流程状态与 handler（从 ExportImportPage 提取）。
 * 覆盖：预览 → 解密 → 选择（对象/页面/附件/展开）→ 冲突策略 → 执行导入。
 */
export function useImportState({
  accountId,
  onError,
  onSuccess,
  t,
  i18n,
  reloadScope,
}: {
  accountId: string;
  onError: (e: Error, fallback: string) => void;
  onSuccess: (msg: string) => void;
  t: TFunction;
  i18n: I18n;
  reloadScope: () => void;
}) {
  const [importPath, setImportPath] = useState('');
  const [stagedImportPath, setStagedImportPath] = useState<string | null>(null);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);

  const [importPw, setImportPw] = useState('');
  const [decryptedPreview, setDecryptedPreview] = useState<DecryptedImportPreview | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isDecrypting, setIsDecrypting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [importStrategy, setImportStrategy] = useState<ImportStrategy>('skipExisting');
  const [importSelections, setImportSelections] = useState<Map<string, boolean>>(new Map());
  const [showStrategySelector, setShowStrategySelector] = useState(false);
  const [importSelectedPageIds, setImportSelectedPageIds] = useState<Set<string>>(new Set());
  const [importSelectedAttachmentIds, setImportSelectedAttachmentIds] = useState<Set<string>>(
    new Set(),
  );
  const [importExpandedPages, setImportExpandedPages] = useState<Set<string>>(new Set());
  const [importExpandedObjects, setImportExpandedObjects] = useState<Set<string>>(new Set());
  const [objectConflictStrategies, setObjectConflictStrategies] = useState<
    Map<string, ImportStrategy>
  >(new Map());

  const handlePreviewImport = async () => {
    if (!importPath || isPreviewing) return;
    setIsPreviewing(true);
    try {
      const sourcePath = await resolveImportSource();
      const preview = await invoke<ImportPreview>('import_parse_package', {
        filePath: sourcePath,
      });
      setImportPreview(preview);
      setDecryptedPreview(null);
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:preview_failed'));
    } finally {
      setIsPreviewing(false);
    }
  };

  const handleDecryptPreview = async () => {
    if (!importPath || !importPw || isDecrypting) return;
    setIsDecrypting(true);
    try {
      const sourcePath = await resolveImportSource();
      const preview = await invoke<DecryptedImportPreview>('import_decrypt_preview', {
        filePath: sourcePath,
        password: importPw,
      });
      setDecryptedPreview(preview);

      // 全选所有对象
      const selMap = new Map<string, boolean>();
      for (const obj of preview.objects) {
        selMap.set(obj.id, true);
      }
      setImportSelections(selMap);

      // 全选所有附件
      const attIds = new Set(preview.attachments.map((a) => a.id));
      setImportSelectedAttachmentIds(attIds);

      // 按 section_type 构建页面全选集合
      const pageIds = new Set<string>();
      for (const obj of preview.objects) {
        const st = obj.sectionType || 'uncategorized';
        pageIds.add(st);
      }
      // 重置冲突策略
      setObjectConflictStrategies(new Map());
      setImportSelectedPageIds(pageIds);
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:decrypt_failed'));
    } finally {
      setIsDecrypting(false);
    }
  };

  const handleImport = async () => {
    if (!importPath || !importPw || importTotalSelected === 0) return;
    setIsImporting(true);
    try {
      const sourcePath = await resolveImportSource();
      const selections = Array.from(importSelections.entries()).map(([objectId, selected]) => ({
        objectId,
        selected,
      }));
      const selAttIds =
        importSelectedAttachmentIds.size > 0 ? Array.from(importSelectedAttachmentIds) : [];

      // 构建 per-object 策略（仅对有显式覆盖设置的冲突对象）
      const objectStrategies: Record<string, ImportStrategy> = {};
      if (decryptedPreview) {
        for (const conflict of decryptedPreview.conflicts) {
          const strategy = objectConflictStrategies.get(conflict.objectId);
          if (strategy && strategy !== importStrategy) {
            objectStrategies[conflict.objectId] = strategy;
          }
        }
      }

      const result = await invoke<ImportResult>('import_execute_advanced', {
        accountId: accountId,
        req: {
          selections,
          strategy: showStrategySelector ? importStrategy : 'skipExisting',
          sourcePath,
          password: importPw,
          selectedAttachmentIds: selAttIds.length > 0 ? selAttIds : null,
          objectStrategies,
          locale: i18n.language,
        },
      });
      onSuccess(
        t('settings:import_success_with_attachments', {
          count: result.objectCount,
          attachments: result.attachmentCount,
        }),
      );
      setImportPreview(null);
      setDecryptedPreview(null);
      setImportPath('');
      if (stagedImportPath) {
        cleanupStagedFile(stagedImportPath);
        setStagedImportPath(null);
      }
      setImportPw('');
      setShowStrategySelector(false);
      setObjectConflictStrategies(new Map());
      reloadScope();
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:import_failed'));
    } finally {
      setIsImporting(false);
    }
  };

  // ── 导入树选择处理 ──

  const toggleImportSelection = (id: string) => {
    setImportSelections((prev) => {
      const next = new Map(prev);
      const newVal = !next.get(id);
      next.set(id, newVal);
      return next;
    });
  };

  const toggleImportPage = (sectionType: string, objectIds: string[]) => {
    setImportSelectedPageIds((prev) => {
      const next = new Set(prev);
      const currentlyChecked = next.has(sectionType);
      if (currentlyChecked) {
        next.delete(sectionType);
      } else {
        next.add(sectionType);
      }
      return next;
    });
    // 同步切换该页面下所有对象的选择状态
    setImportSelections((prev) => {
      const next = new Map(prev);
      const currentlyChecked = importSelectedPageIds.has(sectionType);
      for (const id of objectIds) {
        next.set(id, !currentlyChecked);
      }
      return next;
    });
    // 同步切换该页面下所有附件
    if (decryptedPreview) {
      const pageAttIds = decryptedPreview.attachments
        .filter((a) => objectIds.includes(a.objectId))
        .map((a) => a.id);
      setImportSelectedAttachmentIds((prev) => {
        const next = new Set(prev);
        const currentlyChecked = importSelectedPageIds.has(sectionType);
        for (const attId of pageAttIds) {
          if (currentlyChecked) {
            next.delete(attId);
          } else {
            next.add(attId);
          }
        }
        return next;
      });
    }
  };

  const handleSetObjectConflictStrategy = (objectId: string, strategy: ImportStrategy) => {
    setObjectConflictStrategies((prev) => {
      const next = new Map(prev);
      next.set(objectId, strategy);
      return next;
    });
  };

  const toggleImportAttachment = (attId: string) => {
    setImportSelectedAttachmentIds((prev) => {
      const next = new Set(prev);
      if (next.has(attId)) {
        next.delete(attId);
      } else {
        next.add(attId);
      }
      return next;
    });
  };

  const toggleExpandedImportPage = (sectionType: string) => {
    setImportExpandedPages((prev) => {
      const next = new Set(prev);
      if (next.has(sectionType)) {
        next.delete(sectionType);
      } else {
        next.add(sectionType);
      }
      return next;
    });
  };

  const toggleImportObjectExpanded = (objectId: string) => {
    setImportExpandedObjects((prev) => {
      const next = new Set(prev);
      if (next.has(objectId)) {
        next.delete(objectId);
      } else {
        next.add(objectId);
      }
      return next;
    });
  };

  // 全选/取消全选
  const handleSelectAllImport = useCallback(
    (selectAll: boolean) => {
      if (!decryptedPreview) return;
      const selMap = new Map<string, boolean>();
      for (const obj of decryptedPreview.objects) {
        selMap.set(obj.id, selectAll);
      }
      setImportSelections(selMap);

      if (selectAll) {
        const attIds = new Set(decryptedPreview.attachments.map((a) => a.id));
        setImportSelectedAttachmentIds(attIds);
        const pageIds = new Set<string>();
        for (const obj of decryptedPreview.objects) {
          pageIds.add(obj.sectionType || 'uncategorized');
        }
        setImportSelectedPageIds(pageIds);
      } else {
        setImportSelectedAttachmentIds(new Set());
        setImportSelectedPageIds(new Set());
      }
    },
    [decryptedPreview],
  );

  // 导入总选择数
  const importTotalSelected = useMemo(() => {
    let count = 0;
    for (const v of importSelections.values()) {
      if (v) count++;
    }
    return count;
  }, [importSelections]);

  /**
   * 获取导入命令实际使用的本地路径。
   * Android 返回 content:// URI 时，先通过 plugin-fs 复制到应用缓存。
   */
  const resolveImportSource = useCallback(async () => {
    if (stagedImportPath) return stagedImportPath;
    if (isUriPath(importPath)) {
      const local = await stageImportPackage(importPath);
      setStagedImportPath(local);
      return local;
    }
    return importPath;
  }, [importPath, stagedImportPath]);

  const handleSetImportPath = useCallback(
    (path: string) => {
      setImportPath(path);
      if (stagedImportPath) {
        cleanupStagedFile(stagedImportPath);
        setStagedImportPath(null);
      }
    },
    [stagedImportPath],
  );

  return {
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
    setImportPreview,
    setDecryptedPreview,
    setImportPw,
    setShowStrategySelector,
    setImportStrategy,
    onPreview: handlePreviewImport,
    onDecrypt: handleDecryptPreview,
    onImport: handleImport,
    onSetImportPath: handleSetImportPath,
    onToggleSelection: toggleImportSelection,
    onToggleImportPage: toggleImportPage,
    onToggleImportAttachment: toggleImportAttachment,
    onToggleExpandedImportPage: toggleExpandedImportPage,
    onToggleImportObjectExpanded: toggleImportObjectExpanded,
    onSelectAllImport: handleSelectAllImport,
    onSetObjectConflictStrategy: handleSetObjectConflictStrategy,
  };
}
