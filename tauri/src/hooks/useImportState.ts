import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import type { TFunction, i18n as I18n } from 'i18next';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { resolveI18nPrefix } from '@/lib/utils';
import { cleanupStagedFile, isUriPath, stageImportPackage } from '@/lib/mobileFileTransfer';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { importOutcomeError } from '@/lib/importOutcome';
import type {
  ImportPreview,
  DecryptedImportPreview,
  ImportStrategy,
  ImportResult,
  AdvancedImportRequest,
  ImportOperationSummary,
  ImportOperationsUi,
} from '@/types/exportImport';

type FrozenOptions = Readonly<{
  selections: readonly Readonly<{ objectId: string; selected: boolean }>[];
  strategy: ImportStrategy;
  selectedAttachmentIds: readonly string[];
  objectStrategies: Readonly<Record<string, ImportStrategy>>;
  locale: string;
}>;
type FreshTask = {
  operationId: string;
  sourceVersion: number;
  originalSource: string;
  options: FrozenOptions;
  attempted: boolean;
  accepted: boolean;
};
type SourceCache = { version: number; original: string; path: string };

function isOperationMissing(error: unknown) {
  const raw = error instanceof Error ? error.message : String(error);
  const parsed = resolveI18nPrefix(raw);
  return parsed?.kind === 'import' && parsed.code === 'OPERATION_NOT_FOUND';
}

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
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);

  const [importPw, setImportPw] = useState('');
  const [decryptedPreview, setDecryptedPreview] = useState<DecryptedImportPreview | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isDecrypting, setIsDecrypting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [importStrategy, setImportStrategyState] = useState<ImportStrategy>('skipExisting');
  const [importSelections, setImportSelections] = useState<Map<string, boolean>>(new Map());
  const [showStrategySelector, setShowStrategySelectorState] = useState(false);
  const [importSelectedAttachmentIds, setImportSelectedAttachmentIds] = useState<Set<string>>(
    new Set(),
  );
  const importSelectedPageIds = useMemo(() => {
    const groupedIds = new Map<string, string[]>();
    for (const obj of decryptedPreview?.objects ?? []) {
      const sectionType = obj.sectionType || 'uncategorized';
      const ids = groupedIds.get(sectionType) ?? [];
      ids.push(obj.id);
      groupedIds.set(sectionType, ids);
    }
    const selected = new Set<string>();
    for (const [sectionType, ids] of groupedIds) {
      if (ids.every((id) => importSelections.get(id) === true)) selected.add(sectionType);
    }
    return selected;
  }, [decryptedPreview, importSelections]);
  const [importExpandedPages, setImportExpandedPages] = useState<Set<string>>(new Set());
  const [importExpandedObjects, setImportExpandedObjects] = useState<Set<string>>(new Set());
  const [objectConflictStrategies, setObjectConflictStrategies] = useState<
    Map<string, ImportStrategy>
  >(new Map());
  const sourceVersion = useRef(0);
  const passwordVersion = useRef(0);
  const draftVersion = sourceVersion.current;
  const draftCredentialVersion = passwordVersion.current;
  const optionsVersion = useRef(0);
  const draftOptionsVersion = optionsVersion.current;
  const resumeInputVersion = useRef(0);
  const renderedResumeInputVersion = resumeInputVersion.current;
  const mounted = useRef(true);
  const requests = useMemo(() => createSessionRequests(), []);
  const [sessionEpoch, setSessionEpoch] = useState(0);
  const scope = useMemo(
    () => ({ accountId, epoch: sessionEpoch, ticket: requests.begin(undefined, accountId) }),
    [accountId, sessionEpoch, requests],
  );
  const currentScope = useRef(scope);
  currentScope.current = scope;
  const ownsScope = () =>
    mounted.current && currentScope.current === scope && scope.ticket.isCurrent();
  const cache = useRef<SourceCache | null>(null);
  const leasedCaches = useRef(new Set<string>());
  // 回复丢失时不能断言 worker 已停止读取；保留该输入缓存直到 Native 恢复路径接管。
  const uncertainCaches = useRef(new Set<string>());
  const task = useRef<FreshTask | null>(null);
  const selectedRef = useRef<ImportOperationSummary | null>(null);
  const activeRun = useRef<object | null>(null);
  const [currentId, setCurrentId] = useState<string | null>(null);
  const [operations, setOperations] = useState<ImportOperationSummary[]>([]);
  const [selected, setSelected] = useState<ImportOperationSummary | null>(null);
  const [loadingOperations, setLoadingOperations] = useState(false);
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [resumePassword, setResumePassword] = useState('');
  const [replacementSource, setReplacementSource] = useState('');

  const clearDecryptedState = useCallback(() => {
    setDecryptedPreview(null);
    setImportSelections(new Map());
    setImportSelectedAttachmentIds(new Set());
    setImportExpandedPages(new Set());
    setImportExpandedObjects(new Set());
    setObjectConflictStrategies(new Map());
    setShowStrategySelectorState(false);
    setIsDecrypting(false);
  }, []);

  const detachCache = () => {
    const old = cache.current;
    cache.current = null;
    if (old && !leasedCaches.current.has(old.path) && !uncertainCaches.current.has(old.path)) {
      void cleanupStagedFile(old.path);
    }
  };
  const clearView = () => {
    sourceVersion.current += 1;
    passwordVersion.current += 1;
    requests.invalidate();
    activeRun.current = null;
    task.current = null;
    selectedRef.current = null;
    resumeInputVersion.current += 1;
    detachCache();
    setCurrentId(null);
    setSelected(null);
    setImportPath('');
    setImportPreview(null);
    setImportPw('');
    setResumePassword('');
    setReplacementSource('');
    setIsPreviewing(false);
    setIsDecrypting(false);
    setIsImporting(false);
    setLoadingDetails(false);
    clearDecryptedState();
    setSessionEpoch((value) => value + 1);
  };
  useEffect(() => {
    mounted.current = true;
    const remove = onRequestSessionChange(() => {
      clearView();
      setOperations([]);
      setLoadingOperations(false);
    });
    return () => {
      mounted.current = false;
      sourceVersion.current += 1;
      passwordVersion.current += 1;
      activeRun.current = null;
      task.current = null;
      selectedRef.current = null;
      detachCache();
      remove();
    };
    // clearView 使用稳定 setter/ref；订阅只同步失效，下一次 render 绑定新 scope。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [requests]);

  const discardFreshHandle = () => {
    optionsVersion.current += 1;
    task.current = null;
    selectedRef.current = null;
    setCurrentId(null);
    setSelected(null);
    setResumePassword('');
    setReplacementSource('');
  };
  const canEditDraft = () => ownsScope() && activeRun.current === null;
  const resolveImportSource = async (version: number, original: string): Promise<string | null> => {
    if (!ownsScope() || version !== sourceVersion.current) return null;
    if (cache.current?.version === version && cache.current.original === original) {
      return cache.current.path;
    }
    if (!isUriPath(original)) return original;
    const path = await stageImportPackage(original);
    if (!ownsScope() || version !== sourceVersion.current) {
      void cleanupStagedFile(path);
      return null;
    }
    cache.current = { version, original, path };
    return path;
  };
  const reloadOperations = async () => {
    // 旧闭包不能在 begin 前使当前账户的新列表过期。
    if (!ownsScope()) return;
    const request = requests.begin('operations:list', accountId);
    const isCurrent = () => ownsScope() && request.isCurrent();
    setLoadingOperations(true);
    try {
      const items = await request.invoke<ImportOperationSummary[]>('import_operations_list', {
        accountId,
      });
      if (isCurrent()) setOperations(items);
    } catch (error) {
      if (isCurrent())
        onError(
          new Error(resolveBackendErrorMessage(error)),
          t('settings:import_pending_load_failed'),
        );
    } finally {
      if (isCurrent()) setLoadingOperations(false);
    }
  };
  useEffect(() => {
    void reloadOperations();
    // 每个原账户/会话 epoch 读取一次；不随密码、选项或列表状态重读。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scope]);

  const handleSetImportPw = (password: string) => {
    if (!canEditDraft()) return;
    passwordVersion.current += 1;
    setImportPw(password);
    // 口令变化使预览过期，但不会改已冻结任务的选择或 UUID。
    clearDecryptedState();
  };
  const handlePreviewImport = async () => {
    if (
      !ownsScope() ||
      draftVersion !== sourceVersion.current ||
      !importPath ||
      isPreviewing ||
      activeRun.current
    )
      return;
    const version = sourceVersion.current;
    const request = requests.begin('preview', accountId);
    const isCurrent = () => ownsScope() && request.isCurrent() && version === sourceVersion.current;
    setIsPreviewing(true);
    try {
      const sourcePath = await resolveImportSource(version, importPath);
      if (!sourcePath || !isCurrent()) return;
      const preview = await request.invoke<ImportPreview>('import_parse_package', {
        filePath: sourcePath,
      });
      if (!isCurrent()) return;
      setImportPreview(preview);
      setDecryptedPreview(null);
    } catch (error) {
      if (isCurrent())
        onError(new Error(resolveBackendErrorMessage(error)), t('common:preview_failed'));
    } finally {
      if (isCurrent()) setIsPreviewing(false);
    }
  };
  const handleDecryptPreview = async () => {
    if (
      !ownsScope() ||
      draftVersion !== sourceVersion.current ||
      draftCredentialVersion !== passwordVersion.current ||
      !importPath ||
      !importPw ||
      isDecrypting ||
      activeRun.current
    )
      return;
    const version = sourceVersion.current;
    const credential = passwordVersion.current;
    const password = importPw;
    const request = requests.begin('decrypt', accountId);
    const isCurrent = () =>
      ownsScope() &&
      request.isCurrent() &&
      version === sourceVersion.current &&
      credential === passwordVersion.current;
    setIsDecrypting(true);
    try {
      const sourcePath = await resolveImportSource(version, importPath);
      if (!sourcePath || !isCurrent()) return;
      const preview = await request.invoke<DecryptedImportPreview>('import_decrypt_preview', {
        filePath: sourcePath,
        password,
      });
      if (!isCurrent()) return;
      setDecryptedPreview(preview);
      setImportSelections(new Map(preview.objects.map((object) => [object.id, true])));
      setImportSelectedAttachmentIds(
        new Set(preview.attachments.map((attachment) => attachment.id)),
      );
      setObjectConflictStrategies(new Map());
    } catch (error) {
      if (isCurrent())
        onError(new Error(resolveBackendErrorMessage(error)), t('common:decrypt_failed'));
    } finally {
      if (isCurrent()) setIsDecrypting(false);
    }
  };

  const publishSummary = (summary: ImportOperationSummary) => {
    if (task.current?.operationId !== summary.operationId) task.current = null;
    resumeInputVersion.current += 1;
    passwordVersion.current += 1;
    setImportPw('');
    setDecryptedPreview(null);
    selectedRef.current = summary;
    setSelected(summary);
    setCurrentId(summary.operationId);
    setResumePassword('');
    setReplacementSource('');
    setOperations((items) =>
      summary.phase === 'complete'
        ? items.filter((item) => item.operationId !== summary.operationId)
        : [...items.filter((item) => item.operationId !== summary.operationId), summary],
    );
  };
  const inspectOperation = async (operationId: string) => {
    if (!ownsScope() || activeRun.current) return;
    const request = requests.begin('operations:details', accountId);
    const isCurrent = () => ownsScope() && request.isCurrent();
    setLoadingDetails(true);
    try {
      const summary = await request.invoke<ImportOperationSummary>('import_operation_get', {
        accountId,
        operationId,
      });
      if (isCurrent()) publishSummary(summary);
    } catch (error) {
      if (isCurrent())
        onError(new Error(resolveBackendErrorMessage(error)), t('common:import_failed'));
    } finally {
      if (isCurrent()) setLoadingDetails(false);
    }
  };
  const publishOutcome = (result: ImportResult) => {
    const incomplete = importOutcomeError(result, t);
    if (incomplete) {
      if (result.status === 'partial') reloadScope();
      onError(new Error(incomplete), t('common:import_failed'));
      return false;
    }
    onSuccess(
      t('settings:import_success_with_attachments', {
        count: result.objectCount,
        attachments: result.attachmentCount,
      }),
    );
    reloadScope();
    return true;
  };
  const handleImport = async () => {
    if (
      !ownsScope() ||
      draftVersion !== sourceVersion.current ||
      draftCredentialVersion !== passwordVersion.current ||
      draftOptionsVersion !== optionsVersion.current ||
      activeRun.current
    )
      return;
    let fresh = task.current;
    if (!fresh) {
      if (!importPath || !importPw || importTotalSelected === 0) return;
      const objectStrategies: Record<string, ImportStrategy> = {};
      for (const conflict of decryptedPreview?.conflicts ?? []) {
        const strategy = objectConflictStrategies.get(conflict.objectId);
        if (strategy && strategy !== importStrategy) objectStrategies[conflict.objectId] = strategy;
      }
      // 首次动作在任何 await 前分配 ID 并冻结选择、策略和 locale。
      const options: FrozenOptions = Object.freeze({
        selections: Object.freeze(
          Array.from(importSelections, ([objectId, selected]) =>
            Object.freeze({ objectId, selected }),
          ),
        ),
        strategy: showStrategySelector ? importStrategy : 'skipExisting',
        selectedAttachmentIds: Object.freeze(Array.from(importSelectedAttachmentIds)),
        objectStrategies: Object.freeze(objectStrategies),
        locale: i18n.language,
      });
      fresh = {
        operationId: crypto.randomUUID(),
        sourceVersion: sourceVersion.current,
        originalSource: importPath,
        options,
        attempted: false,
        accepted: false,
      };
      task.current = fresh;
      setCurrentId(fresh.operationId);
    }
    const operation = fresh;
    const password = importPw;
    const run = {};
    requests.invalidate('operations:details');
    requests.invalidate('resume:source');
    setLoadingDetails(false);
    const request = requests.begin('run', accountId);
    activeRun.current = run;
    const isCurrent = () =>
      ownsScope() && request.isCurrent() && activeRun.current === run && task.current === operation;
    setIsImporting(true);
    let localSource: string | null = null;
    let outcomeKnown = false;
    try {
      if (operation.attempted) {
        try {
          const summary = await request.invoke<ImportOperationSummary>('import_operation_get', {
            accountId,
            operationId: operation.operationId,
          });
          if (!isCurrent()) return;
          operation.accepted = true;
          if (summary.phase === 'complete') {
            publishOutcome(summary.outcome);
            clearView();
          } else publishSummary(summary);
          return; // 继续入口按摘要索取必要材料，不复用旧预览 gating。
        } catch (error) {
          if (!isCurrent()) return;
          if (operation.accepted || !isOperationMissing(error)) throw error;
          // 未登记的 unknown 回复重试复用同 UUID 和原冻结选项。
        }
      }
      if (!password) return;
      localSource = await resolveImportSource(operation.sourceVersion, operation.originalSource);
      if (!localSource || !isCurrent()) return;
      if (isUriPath(operation.originalSource)) leasedCaches.current.add(localSource);
      const req: AdvancedImportRequest = {
        operationId: operation.operationId,
        selections: operation.options.selections.map((selection) => ({ ...selection })),
        strategy: operation.options.strategy,
        sourcePath: localSource,
        password,
        selectedAttachmentIds: [...operation.options.selectedAttachmentIds],
        objectStrategies: { ...operation.options.objectStrategies },
        locale: operation.options.locale,
      };
      operation.attempted = true;
      const result = await request.invoke<ImportResult>('import_execute_advanced', {
        accountId,
        req,
      });
      outcomeKnown = true;
      if (!isCurrent()) return;
      operation.accepted = result.status === 'partial' || result.status === 'complete';
      if (result.operationId && result.operationId !== operation.operationId)
        throw new Error('__IMPORT_ERR__:OPERATION_CONFLICT');
      if (publishOutcome(result)) {
        setOperations((items) =>
          items.filter((item) => item.operationId !== operation.operationId),
        );
        clearView();
      } else if (operation.accepted) {
        const summary = await request.invoke<ImportOperationSummary>('import_operation_get', {
          accountId,
          operationId: operation.operationId,
        });
        if (isCurrent()) publishSummary(summary);
      }
    } catch (error) {
      if (
        !outcomeKnown &&
        operation.attempted &&
        localSource &&
        isUriPath(operation.originalSource)
      )
        uncertainCaches.current.add(localSource);
      if (isCurrent())
        onError(new Error(resolveBackendErrorMessage(error)), t('common:import_failed'));
    } finally {
      if (localSource && isUriPath(operation.originalSource)) {
        leasedCaches.current.delete(localSource);
        if (
          !isCurrent() &&
          !uncertainCaches.current.has(localSource) &&
          cache.current?.path !== localSource
        )
          void cleanupStagedFile(localSource);
      }
      if (isCurrent()) {
        activeRun.current = null;
        setIsImporting(false);
      }
    }
  };

  const handleResume = async () => {
    if (
      !ownsScope() ||
      renderedResumeInputVersion !== resumeInputVersion.current ||
      activeRun.current ||
      !selectedRef.current
    )
      return;
    const summary = selectedRef.current;
    if (
      summary.phase === 'complete' ||
      (summary.passwordRequired && !resumePassword) ||
      (summary.sourceRequired && !replacementSource)
    )
      return;
    // 已接纳 recovery 应已 ready；异常 credential 提示不能诱导输入新主密码代替内部口令。
    if (summary.sourceKind === 'recovery' && summary.passwordRequired) return;
    const password = summary.passwordRequired ? resumePassword : null;
    const source = summary.sourceRequired ? replacementSource : null;
    const run = {};
    requests.invalidate('operations:details');
    requests.invalidate('resume:source');
    setLoadingDetails(false);
    const request = requests.begin('run', accountId);
    activeRun.current = run;
    const isCurrent = () =>
      ownsScope() &&
      request.isCurrent() &&
      activeRun.current === run &&
      selectedRef.current?.operationId === summary.operationId;
    setIsImporting(true);
    let staged: string | null = null;
    let outcomeKnown = false;
    try {
      let sourcePath = source;
      if (source && isUriPath(source)) {
        staged = await stageImportPackage(source);
        if (!isCurrent()) return;
        leasedCaches.current.add(staged);
        sourcePath = staged;
      }
      if (!isCurrent()) return;
      const result = await request.invoke<ImportResult>('import_operation_resume', {
        accountId,
        operationId: summary.operationId,
        password,
        sourcePath,
      });
      outcomeKnown = true;
      if (!isCurrent()) return;
      if (result.operationId && result.operationId !== summary.operationId)
        throw new Error('__IMPORT_ERR__:OPERATION_CONFLICT');
      setResumePassword('');
      if (publishOutcome(result)) {
        setOperations((items) => items.filter((item) => item.operationId !== summary.operationId));
        clearView();
      } else {
        const updated = await request.invoke<ImportOperationSummary>('import_operation_get', {
          accountId,
          operationId: summary.operationId,
        });
        if (isCurrent()) publishSummary(updated);
      }
    } catch (error) {
      if (isCurrent()) {
        if (!outcomeKnown && staged) uncertainCaches.current.add(staged);
        onError(new Error(resolveBackendErrorMessage(error)), t('common:import_failed'));
      }
    } finally {
      if (staged) {
        leasedCaches.current.delete(staged);
        if (!uncertainCaches.current.has(staged)) void cleanupStagedFile(staged);
      }
      if (isCurrent()) {
        activeRun.current = null;
        setIsImporting(false);
      }
    }
  };
  const pickReplacementSource = async () => {
    if (!ownsScope() || activeRun.current || !selectedRef.current) return;
    const target = selectedRef.current;
    const request = requests.begin('resume:source', accountId);
    const isCurrent = () =>
      ownsScope() &&
      request.isCurrent() &&
      selectedRef.current === target &&
      activeRun.current === null;
    const { openWithPause } = await import('@/lib/dialog');
    if (!isCurrent()) return;
    const chosen = await openWithPause({
      filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
      multiple: false,
    });
    if (isCurrent() && typeof chosen === 'string') {
      resumeInputVersion.current += 1;
      setReplacementSource(chosen);
    }
  };
  const leaveView = () => {
    if (!ownsScope()) return;
    // 只撤销当前视图和口令；有效派发的 Native worker 与持久任务不会被取消。
    clearView();
  };
  const handleSetImportPath = (path: string) => {
    if (!canEditDraft()) return;
    sourceVersion.current += 1;
    passwordVersion.current += 1;
    detachCache();
    discardFreshHandle();
    setImportPath(path);
    setImportPreview(null);
    setImportPw('');
    setIsPreviewing(false);
    clearDecryptedState();
  };
  const setImportStrategy = (strategy: ImportStrategy) => {
    if (!canEditDraft()) return;
    discardFreshHandle();
    setImportStrategyState(strategy);
  };
  const setShowStrategySelector = (show: boolean) => {
    if (!canEditDraft()) return;
    discardFreshHandle();
    setShowStrategySelectorState(show);
  };
  // ── 导入树选择处理 ──

  const toggleImportSelection = (id: string) => {
    if (!canEditDraft()) return;
    discardFreshHandle();
    setImportSelections((prev) => {
      const next = new Map(prev);
      const newVal = !next.get(id);
      next.set(id, newVal);
      if (decryptedPreview) {
        const attachmentIds = decryptedPreview.attachments
          .filter((attachment) => attachment.objectId === id)
          .map((attachment) => attachment.id);
        setImportSelectedAttachmentIds((selected) => {
          const updated = new Set(selected);
          for (const attachmentId of attachmentIds) {
            if (newVal) updated.add(attachmentId);
            else updated.delete(attachmentId);
          }
          return updated;
        });
      }
      return next;
    });
  };

  const toggleImportPage = (sectionType: string, objectIds: string[]) => {
    if (!canEditDraft()) return;
    discardFreshHandle();
    const currentlyChecked = importSelectedPageIds.has(sectionType);
    // 同步切换该页面下所有对象的选择状态
    setImportSelections((prev) => {
      const next = new Map(prev);
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
    if (!canEditDraft()) return;
    discardFreshHandle();
    setObjectConflictStrategies((prev) => {
      const next = new Map(prev);
      next.set(objectId, strategy);
      return next;
    });
  };

  const toggleImportAttachment = (attId: string) => {
    if (!canEditDraft()) return;
    discardFreshHandle();
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
    if (!ownsScope()) return;
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
    if (!ownsScope()) return;
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
  const handleSelectAllImport = (selectAll: boolean) => {
    if (!canEditDraft() || !decryptedPreview) return;
    discardFreshHandle();
    const selMap = new Map<string, boolean>();
    for (const obj of decryptedPreview.objects) {
      selMap.set(obj.id, selectAll);
    }
    setImportSelections(selMap);

    if (selectAll) {
      const attIds = new Set(decryptedPreview.attachments.map((a) => a.id));
      setImportSelectedAttachmentIds(attIds);
    } else {
      setImportSelectedAttachmentIds(new Set());
    }
  };

  // 导入总选择数
  const importTotalSelected = useMemo(() => {
    let count = 0;
    for (const v of importSelections.values()) {
      if (v) count++;
    }
    return count;
  }, [importSelections]);

  const importOperations: ImportOperationsUi = {
    items: operations,
    selected,
    currentId,
    loading: loadingOperations,
    loadingDetails,
    busy: isImporting,
    password: resumePassword,
    replacementSource,
    canResume:
      !!selected &&
      selected.phase !== 'complete' &&
      !isImporting &&
      !loadingDetails &&
      !(selected.sourceKind === 'recovery' && selected.passwordRequired) &&
      (!selected.passwordRequired || !!resumePassword) &&
      (!selected.sourceRequired || !!replacementSource),
    onRefresh: reloadOperations,
    onSelect: inspectOperation,
    onRetry: handleImport,
    onResume: () => (selectedRef.current === selected ? handleResume() : Promise.resolve()),
    onSetPassword: (value) => {
      if (ownsScope() && !activeRun.current && selectedRef.current === selected) {
        resumeInputVersion.current += 1;
        setResumePassword(value);
      }
    },
    onPickSource: () =>
      selectedRef.current === selected ? pickReplacementSource() : Promise.resolve(),
    onContinueLater: () => {
      if (
        selectedRef.current === selected &&
        (selected || task.current?.operationId === currentId) &&
        draftVersion === sourceVersion.current
      )
        leaveView();
    },
    onNewImport: () => {
      if (
        selectedRef.current === selected &&
        (selected || task.current?.operationId === currentId) &&
        draftVersion === sourceVersion.current
      )
        leaveView();
    },
  };
  return {
    importOperations,
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
    setImportPw: handleSetImportPw,
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
