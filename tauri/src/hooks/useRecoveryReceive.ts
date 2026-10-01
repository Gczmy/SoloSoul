import { useEffect, useRef, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useNavigate } from 'react-router-dom';
import { useAuthStore, saveLastAccountId } from '@/stores/authStore';
import { useCameraCapability } from '@/hooks/useCameraCapability';
import { useRecoveryManualForm } from '@/hooks/useRecoveryManualForm';
import { useRecoveryCredentials } from '@/hooks/useRecoveryCredentials';
import { friendlyConnectError, checkRecoveryIdConflict } from '@/lib/recoveryErrors';
import { importOutcomeError } from '@/lib/importOutcome';
import { createSessionRequests } from '@/lib/sessionRequests';
import type {
  RecoveryResultSummary,
  ScannedRecoveryQr,
  TabMode,
  Step,
} from '@/components/recovery/recoveryReceiveTypes';

export interface UseRecoveryReceiveOptions {
  isOpen: boolean;
  onClose: () => void;
  /** 恢复成功后调用；若提供则替代默认的首页导航 */
  onSuccess?: () => void;
  /** 已解锁同步页薄入口：此模式没有创建或覆盖账户分支。 */
  existingAccountOnly?: boolean;
}

/** RecoveryReceiveDialog 的完整状态机与业务逻辑。 */
export function useRecoveryReceive({
  isOpen,
  onClose,
  onSuccess,
  existingAccountOnly = false,
}: UseRecoveryReceiveOptions) {
  const { t } = useTranslation(['common', 'settings']);
  const navigate = useNavigate();
  const mountedRef = useRef(true);
  // 后端恢复没有取消入口；关闭弹窗不能把仍在执行的导入隐藏起来。
  const recoveryInFlightRef = useRef(false);
  // 扫码预检可能跨越多次扫描或一次对话框关闭；旧结果不得重写当前选择。
  const scanGenerationRef = useRef(0);
  const requestRef = useRef(createSessionRequests());
  const activeRunRef = useRef(0);
  const unlockedAccountId = useAuthStore((state) =>
    state.isAuthenticated ? (state.currentAccount?.id ?? null) : null,
  );
  // 设备摄像头能力（启动时预加载，模块级缓存）。
  // 支持 → 默认「扫描二维码」；不支持 → 默认「手动输入」。
  const cameraCapability = useCameraCapability();
  // 用户在本次打开期间是否手动切换过 tab（手动切换后不再被默认 tab 覆盖）
  const userSwitchedTabRef = useRef(false);

  // 按设备能力计算默认 tab（支持/未知 → 扫码；不支持 → 手动输入）
  const getDefaultTab = useCallback(
    (): TabMode => (cameraCapability === 'unsupported' ? 'manual' : 'scan'),
    [cameraCapability],
  );

  // 流程状态
  const [step, setStep] = useState<Step>('collect');
  const [tab, setTab] = useState<TabMode>(getDefaultTab);

  // 打开对话框时按设备能力设置默认 tab（尊重用户手动选择）
  useEffect(() => {
    if (isOpen && cameraCapability !== 'unknown' && !userSwitchedTabRef.current) {
      setTab(getDefaultTab());
    }
  }, [isOpen, cameraCapability, getDefaultTab]);

  // 共享状态
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<RecoveryResultSummary | null>(null);
  // 扫码器启动失败（如权限被拒）时置位，用于展示「使用手动输入」兜底按钮
  const [scannerError, setScannerError] = useState<string | null>(null);

  // 恢复执行进度（recovery-progress 事件）：{ phase, percent }，未开始/完成后为 null
  const [progress, setProgress] = useState<{ phase: string; percent: number } | null>(null);
  // 恢复完成后「恢复完成」确认弹窗是否打开
  const [successConfirmOpen, setSuccessConfirmOpen] = useState(false);

  // 已收集的连接信息（扫码或手动输入后统一进入账户卡阶段）
  const [pending, setPending] = useState<ScannedRecoveryQr | null>(null);
  // 账户 ID 冲突（本设备已存在相同 account_id）→ 展示覆盖恢复选项
  const [idConflict, setIdConflict] = useState(false);
  // 用户已确认覆盖恢复 → 展示密码输入（覆盖模式），开始恢复时携带 overwrite=true
  const [overwriteApproved, setOverwriteApproved] = useState(false);
  // 二次确认覆盖弹窗是否打开
  const [confirmingOverwrite, setConfirmingOverwrite] = useState(false);
  // 连接/传输中的状态文案（账户卡展示）
  const [statusText, setStatusText] = useState<string | null>(null);
  // Fresh 入口由用户显式选择；已接纳任务与尚未提交的重接收绝不混用。
  const [existingAccountId, setExistingAccountId] = useState<string | null>(null);
  const [retryTargetAccountId, setRetryTargetAccountId] = useState<string | null>(null);
  const [acceptedRecovery, setAcceptedRecovery] = useState<RecoveryResultSummary | null>(null);
  const [connectionConsumed, setConnectionConsumed] = useState(false);
  const [completionUsesExistingAccount, setCompletionUsesExistingAccount] = useState(false);
  const canImportExisting = Boolean(
    unlockedAccountId &&
    (!pending?.accountId || pending.accountId === unlockedAccountId) &&
    (!retryTargetAccountId || retryTargetAccountId === unlockedAccountId),
  );
  const existingAccountUnlocked =
    existingAccountId !== null && existingAccountId === unlockedAccountId;
  const canResumeRecovery = Boolean(
    acceptedRecovery?.operationId && acceptedRecovery.accountId === unlockedAccountId,
  );

  // 子 hook：手动连接表单 + 局域网发现（collect 阶段 manual tab）
  const manualForm = useRecoveryManualForm({ mountedRef });
  // 子 hook：账户阶段的主密码表单与校验
  const credentials = useRecoveryCredentials({
    onEdited: () => {
      setError(null);
      // 覆盖模式（overwriteApproved）下不清除 idConflict——密码输入框仅在确认覆盖后渲染，
      // 若在此清冲突会误切换到普通分支，导致 UI 显示与 overwrite=true 实际行为不一致。
      if (idConflict && !overwriteApproved) setIdConflict(false);
    },
  });

  useEffect(() => {
    const requests = requestRef.current;
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      requests.invalidate();
      activeRunRef.current += 1;
    };
  }, []);

  useEffect(() => {
    if (!isOpen) {
      scanGenerationRef.current += 1;
      requestRef.current.invalidate();
      activeRunRef.current += 1;
    }
  }, [isOpen]);

  // ── 重置所有状态 ──
  const resetState = () => {
    scanGenerationRef.current += 1;
    requestRef.current.invalidate();
    activeRunRef.current += 1;
    setExistingAccountId(null);
    setRetryTargetAccountId(null);
    setAcceptedRecovery(null);
    setConnectionConsumed(false);
    setCompletionUsesExistingAccount(false);
    setStep('collect');
    setTab(getDefaultTab());
    userSwitchedTabRef.current = false;
    setError(null);
    setSuccess(null);
    setProgress(null);
    setSuccessConfirmOpen(false);
    setScannerError(null);
    setLoading(false);
    setPending(null);
    setIdConflict(false);
    setOverwriteApproved(false);
    setConfirmingOverwrite(false);
    setStatusText(null);
    manualForm.reset();
    credentials.reset();
  };

  const handleClose = () => {
    if (recoveryInFlightRef.current) return;
    if (success) {
      if (onSuccess) {
        onSuccess();
      } else {
        navigate('/', { replace: true });
      }
    }
    resetState();
    onClose();
  };

  // ── Tab 切换 ──
  const switchTab = (newTab: TabMode) => {
    if (loading) return; // 传输中禁止切换
    scanGenerationRef.current += 1;
    userSwitchedTabRef.current = true;
    setError(null);
    setScannerError(null);
    setTab(newTab);
  };

  // ── 扫码：解析 t:"rec" 二维码 → 预检账户 ID 冲突 → 进入账户卡 ──
  const handleScan = async (text: string) => {
    if (recoveryInFlightRef.current || acceptedRecovery) return;
    const scanGeneration = ++scanGenerationRef.current;
    try {
      const parsed = JSON.parse(text);
      if (parsed.t !== 'rec') {
        setError(
          t('common:recovery_qr_invalid_reverse', {
            defaultValue:
              'Invalid QR code. Please scan the recovery QR shown on your old device (Settings → Device Sync → Show Recovery QR).',
          }),
        );
        return;
      }
      if (!parsed.a || !parsed.p) {
        setError(t('common:sync_qr_invalid_payload'));
        return;
      }
      setError(null);
      // 扫描完成后立即预检账户 ID 冲突（在进入密码输入之前提示覆盖选项）
      const conflict = await checkRecoveryIdConflict(parsed.u);
      if (!mountedRef.current || scanGeneration !== scanGenerationRef.current) return;
      if (
        (retryTargetAccountId && parsed.u && parsed.u !== retryTargetAccountId) ||
        (existingAccountOnly && parsed.u && parsed.u !== unlockedAccountId)
      ) {
        setError(t('common:recovery_existing_account_mismatch'));
        return;
      }
      setIdConflict(conflict || Boolean(retryTargetAccountId));
      setConnectionConsumed(false);
      setExistingAccountId(existingAccountOnly ? unlockedAccountId : null);
      setOverwriteApproved(false);
      setPending({
        addr: parsed.a,
        pin: parsed.p,
        fingerprint: parsed.f || '',
        nonce: parsed.n || null,
        accountId: parsed.u,
        accountName: parsed.m,
      });
      setStep('account');
    } catch {
      setError(t('common:sync_qr_invalid_payload'));
    }
  };

  // ── 手动输入完成：校验后进入账户卡 ──
  const handleManualNext = () => {
    scanGenerationRef.current += 1;
    setError(null);
    const result = manualForm.getPendingInfo();
    if ('error' in result) {
      setError(result.error);
      return;
    }
    if (recoveryInFlightRef.current || acceptedRecovery) return;
    setPending(result.value);
    setConnectionConsumed(false);
    setExistingAccountId(existingAccountOnly ? unlockedAccountId : null);
    setIdConflict(Boolean(retryTargetAccountId));
    setStep('account');
  };

  const handleUseExistingAccount = () => {
    if (recoveryInFlightRef.current || acceptedRecovery || !canImportExisting || !unlockedAccountId)
      return;
    setExistingAccountId(unlockedAccountId);
    setOverwriteApproved(false);
    setConfirmingOverwrite(false);
    credentials.reset();
    setError(null);
  };

  const applyRecoveryResult = async (result: RecoveryResultSummary, isCurrent: () => boolean) => {
    if (!mountedRef.current || !isCurrent()) return;
    setPending((previous) =>
      previous
        ? { ...previous, accountId: result.accountId, accountName: result.accountName }
        : previous,
    );
    const incomplete = importOutcomeError(result, t);
    if (incomplete) {
      if (result.status === 'partial' && result.operationId) {
        setAcceptedRecovery(result);
        setError(`${incomplete} ${t('common:recovery_ready_resume_note')}`);
      } else if (result.status === 'notCommitted') {
        // 包已被一次性传输消费；口令不保存，也不能把新包用作旧 ID 的 Resume。
        setRetryTargetAccountId(result.accountId);
        setError(`${incomplete} ${t('common:recovery_fresh_retry_note')}`);
      } else {
        setError(incomplete);
      }
      await useAuthStore.getState().checkHasAccount();
      await useAuthStore.getState().listAccounts();
      return;
    }
    await useAuthStore.getState().checkHasAccount();
    await useAuthStore.getState().listAccounts();
    if (!mountedRef.current || !isCurrent()) return;
    setAcceptedRecovery(null);
    setSuccess(result);
    setStep('success');
    saveLastAccountId(result.accountId);
    setSuccessConfirmOpen(true);
  };

  // 新账户模式保留原密码/覆盖契约；existing 模式只连接已解锁同账户。
  const handleStartRecovery = async () => {
    if (!pending || recoveryInFlightRef.current || acceptedRecovery) return;
    if (connectionConsumed) {
      setError(t('common:recovery_new_host_required'));
      return;
    }
    if (existingAccountId) {
      if (!existingAccountUnlocked) {
        setError(t('common:recovery_existing_unlock_required'));
        return;
      }
    } else {
      // 未提交后保留下来的账户只能走显式 Fresh，不再次创建/覆盖。
      if (retryTargetAccountId || existingAccountOnly) {
        setError(t('common:recovery_existing_unlock_required'));
        return;
      }
      if (credentials.getValidationError()) return;
    }
    setError(null);
    setSuccess(null);
    recoveryInFlightRef.current = true;
    setCompletionUsesExistingAccount(Boolean(existingAccountId));
    const run = ++activeRunRef.current;
    const ticket = requestRef.current.begin('recovery', existingAccountId ?? undefined);
    const isCurrent = () => ticket.isCurrent() && run === activeRunRef.current;
    setLoading(true);
    setProgress(null);
    setStatusText(t('common:recovery_connecting', { defaultValue: 'Connecting to host…' }));
    // 进度只影响本次可见请求；退订前排队的旧回调也必须经过 run/session 校验。
    const unlistenPromise = listen<{ phase: string; percent: number }>('recovery-progress', (e) => {
      if (mountedRef.current && recoveryInFlightRef.current && isCurrent()) setProgress(e.payload);
    }).catch(() => null);
    try {
      setConnectionConsumed(true);
      const connection = {
        hostAddr: pending.addr,
        pin: pending.pin,
        fingerprint: pending.fingerprint || null,
        nonce: pending.nonce,
      };
      const result = existingAccountId
        ? await ticket.invoke<RecoveryResultSummary>('recovery_restore_existing_from_host', {
            accountId: existingAccountId,
            ...connection,
          })
        : await ticket.invoke<RecoveryResultSummary>('recovery_restore_from_host', {
            ...connection,
            masterPassword: credentials.masterPassword,
            passwordHint: credentials.passwordHint.trim() || null,
            overwrite: overwriteApproved,
          });
      await applyRecoveryResult(result, isCurrent);
    } catch (err) {
      if (!mountedRef.current || !isCurrent()) return;
      // Native 异常也可能发生于创建账户之后；刷新目录保全登录入口，结果仍保持未知。
      await Promise.allSettled([
        useAuthStore.getState().checkHasAccount(),
        useAuthStore.getState().listAccounts(),
      ]);
      if (!mountedRef.current || !isCurrent()) return;
      const raw = String(err);
      if (raw.includes('Account ID already exists') && !existingAccountId) {
        setIdConflict(true);
        setOverwriteApproved(false);
      } else if (raw.includes('RECOVERY_ACCOUNT_MISMATCH')) {
        setError(t('common:recovery_existing_account_mismatch'));
      } else {
        setError(`${friendlyConnectError(raw, t)} ${t('common:recovery_new_host_required')}`);
      }
    } finally {
      void unlistenPromise.then((un) => un?.());
      recoveryInFlightRef.current = false;
      if (mountedRef.current) {
        setLoading(false);
        setStatusText(null);
      }
    }
  };

  // 已接纳 Recovery 的所有材料已由账户密钥保护；不再下载或询问内部随机包口令。
  const handleResumeRecovery = async () => {
    if (!acceptedRecovery?.operationId || recoveryInFlightRef.current) return;
    if (!canResumeRecovery) {
      setError(t('common:recovery_existing_unlock_required'));
      return;
    }
    recoveryInFlightRef.current = true;
    setCompletionUsesExistingAccount(true);
    const run = ++activeRunRef.current;
    const ticket = requestRef.current.begin('recovery', acceptedRecovery.accountId);
    const isCurrent = () => ticket.isCurrent() && run === activeRunRef.current;
    setLoading(true);
    setError(null);
    setStatusText(t('common:recovery_progress_import'));
    try {
      const outcome = await ticket.invokeTyped('import_operation_resume', {
        accountId: acceptedRecovery.accountId,
        operationId: acceptedRecovery.operationId,
        sourcePath: null,
        password: null,
      });
      await applyRecoveryResult({ ...acceptedRecovery, ...outcome }, isCurrent);
    } catch (err) {
      if (mountedRef.current && isCurrent()) setError(friendlyConnectError(String(err), t));
    } finally {
      recoveryInFlightRef.current = false;
      if (mountedRef.current) {
        setLoading(false);
        setStatusText(null);
      }
    }
  };

  // ── 冲突处理：打开/关闭覆盖二次确认 ──
  const handleRequestOverwrite = () => {
    if (
      recoveryInFlightRef.current ||
      existingAccountOnly ||
      retryTargetAccountId ||
      acceptedRecovery ||
      existingAccountId ||
      connectionConsumed
    )
      return;
    setConfirmingOverwrite(true);
  };

  const handleCancelOverwriteConfirm = () => {
    setConfirmingOverwrite(false);
  };

  // 覆盖二次确认通过：进入覆盖模式 → 展示密码输入，由 handleStartRecovery 携带 overwrite=true 发起
  const handleOverwriteRecovery = () => {
    if (
      recoveryInFlightRef.current ||
      existingAccountOnly ||
      retryTargetAccountId ||
      acceptedRecovery ||
      existingAccountId ||
      connectionConsumed
    )
      return;
    setConfirmingOverwrite(false);
    setOverwriteApproved(true);
  };

  // 冲突警示框「取消」：返回二维码扫描/手动输入卡片页面（放弃恢复）
  const handleCancelConflict = () => {
    handleBackToCollect();
  };

  // ── 账户卡：返回重新获取连接信息 ──
  const handleBackToCollect = () => {
    if (recoveryInFlightRef.current || acceptedRecovery) return;
    scanGenerationRef.current += 1;
    setExistingAccountId(null);
    setConnectionConsumed(false);
    manualForm.reset();
    setPending(null);
    setIdConflict(false);
    setOverwriteApproved(false);
    setConfirmingOverwrite(false);
    setError(null);
    credentials.reset();
    setStep('collect');
  };

  return {
    step,
    tab,
    cameraCapability,
    loading,
    error,
    progress,
    success,
    successConfirmOpen,
    setSuccessConfirmOpen,
    scannerError,
    pending,
    masterPassword: credentials.masterPassword,
    confirmPassword: credentials.confirmPassword,
    passwordHint: credentials.passwordHint,
    masterPasswordError: credentials.masterPasswordError,
    confirmPasswordError: credentials.confirmPasswordError,
    hostAddr: manualForm.hostAddr,
    pin: manualForm.pin,
    fingerprint: manualForm.fingerprint,
    showAdvanced: manualForm.showAdvanced,
    statusText,
    scanning: manualForm.scanning,
    discoveredHosts: manualForm.discoveredHosts,
    scanError: manualForm.scanError,
    scanDone: manualForm.scanDone,
    setPasswordHint: credentials.setPasswordHint,
    setHostAddr: manualForm.setHostAddr,
    setPin: manualForm.setPin,
    setFingerprint: manualForm.setFingerprint,
    setShowAdvanced: manualForm.setShowAdvanced,
    setScannerError,
    handleMasterPasswordChange: credentials.handleMasterPasswordChange,
    handleConfirmPasswordChange: credentials.handleConfirmPasswordChange,
    handleClose,
    switchTab,
    handleScan,
    handleScanLan: manualForm.handleScanLan,
    handleSelectHost: manualForm.handleSelectHost,
    handleManualNext,
    handleStartRecovery,
    handleBackToCollect,
    completionUsesExistingAccount,
    existingAccountId,
    existingAccountUnlocked,
    canImportExisting,
    retryTargetAccountId,
    acceptedRecovery,
    canResumeRecovery,
    connectionConsumed,
    handleUseExistingAccount,
    handleResumeRecovery,
    idConflict,
    overwriteApproved,
    confirmingOverwrite,
    handleOverwriteRecovery,
    handleRequestOverwrite,
    handleCancelConflict,
    handleCancelOverwriteConfirm,
  };
}
