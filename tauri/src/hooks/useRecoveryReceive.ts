import { useEffect, useRef, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useNavigate } from 'react-router-dom';
import { useAuthStore, saveLastAccountId } from '@/stores/authStore';
import { useCameraCapability } from '@/hooks/useCameraCapability';
import { useRecoveryManualForm } from '@/hooks/useRecoveryManualForm';
import { useRecoveryCredentials } from '@/hooks/useRecoveryCredentials';
import { friendlyConnectError, checkRecoveryIdConflict } from '@/lib/recoveryErrors';
import type {
  RecoveryResultSummary,
  ScannedRecoveryQr,
  TabMode,
  Step,
} from '@/components/recovery/recoveryReceiveTypes';

export interface UseRecoveryReceiveOptions {
  isOpen: boolean;
  onClose: () => void;
  /** 恢复成功后调用；若提供则替代默认的 /home 导航 */
  onSuccess?: () => void;
}

/** RecoveryReceiveDialog 的完整状态机与业务逻辑。 */
export function useRecoveryReceive({ isOpen, onClose, onSuccess }: UseRecoveryReceiveOptions) {
  const { t } = useTranslation(['common']);
  const navigate = useNavigate();
  const mountedRef = useRef(true);
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
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // ── 重置所有状态 ──
  const resetState = () => {
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
    if (success) {
      if (onSuccess) {
        onSuccess();
      } else {
        navigate('/home', { replace: true });
      }
    }
    resetState();
    onClose();
  };

  // ── Tab 切换 ──
  const switchTab = (newTab: TabMode) => {
    if (loading) return; // 传输中禁止切换
    userSwitchedTabRef.current = true;
    setError(null);
    setScannerError(null);
    setTab(newTab);
  };

  // ── 扫码：解析 t:"rec" 二维码 → 预检账户 ID 冲突 → 进入账户卡 ──
  const handleScan = async (text: string) => {
    try {
      const parsed = JSON.parse(text);
      if (parsed.t !== 'rec') {
        setError(
          t('common:recovery_qr_invalid_reverse', {
            defaultValue: 'Invalid QR code. Please scan the recovery QR shown on your old device (Settings → Device Sync → Show Recovery QR).',
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
      if (!mountedRef.current) return;
      setIdConflict(conflict);
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
    setError(null);
    const result = manualForm.getPendingInfo();
    if ('error' in result) {
      setError(result.error);
      return;
    }
    setPending(result.value);
    setStep('account');
  };

  // ── 账户卡：设置主密码后开始恢复（与扫码路径走同一命令） ──
  const handleStartRecovery = async () => {
    if (!pending) return;
    setError(null);
    setSuccess(null);

    // 校验（优先级与创建账户页一致）；失败时对应输入框已置 error，直接返回
    if (credentials.getValidationError()) return;

    setLoading(true);
    setProgress(null);
    setStatusText(t('common:recovery_connecting', { defaultValue: 'Connecting to host…' }));

    // 订阅恢复进度事件（recovery-progress：download/overwrite/create/import/done 分阶段百分比）
    const unlistenPromise = listen<{ phase: string; percent: number }>('recovery-progress', (e) => {
      if (mountedRef.current) setProgress(e.payload);
    }).catch(() => null);

    try {
      const result = await invoke<RecoveryResultSummary>('recovery_restore_from_host', {
        hostAddr: pending.addr,
        pin: pending.pin,
        masterPassword: credentials.masterPassword,
        passwordHint: credentials.passwordHint.trim() || null,
        fingerprint: pending.fingerprint || null,
        nonce: pending.nonce,
        overwrite: overwriteApproved,
      });
      if (!mountedRef.current) return;
      setSuccess(result);
      setStep('success');
      // 记住最近恢复的账户：返回登录页后自动选中它，方便用新密码解锁
      saveLastAccountId(result.accountId);
      await useAuthStore.getState().checkHasAccount();
      // 自动弹出「恢复完成」确认框，用户确认后返回登录页
      if (mountedRef.current) setSuccessConfirmOpen(true);
    } catch (err) {
      if (!mountedRef.current) return;
      const raw = String(err);
      if (raw.includes('Account ID already exists')) {
        // 兜底（手动输入等无 accountId 预检的路径）：进入冲突状态，展示覆盖恢复选项（不显示普通错误）
        setIdConflict(true);
        setOverwriteApproved(false);
      } else {
        setError(friendlyConnectError(raw, t));
      }
    } finally {
      // 无论成败都退订进度事件，避免泄漏监听器
      if (unlistenPromise) {
        void unlistenPromise.then((un) => un?.());
      }
      setLoading(false);
      setStatusText(null);
    }
  };

  // ── 冲突处理：打开/关闭覆盖二次确认 ──
  const handleRequestOverwrite = () => {
    if (loading) return;
    setConfirmingOverwrite(true);
  };

  const handleCancelOverwriteConfirm = () => {
    setConfirmingOverwrite(false);
  };

  // 覆盖二次确认通过：进入覆盖模式 → 展示密码输入，由 handleStartRecovery 携带 overwrite=true 发起
  const handleOverwriteRecovery = () => {
    if (loading) return;
    setConfirmingOverwrite(false);
    setOverwriteApproved(true);
  };

  // 冲突警示框「取消」：返回二维码扫描/手动输入卡片页面（放弃恢复）
  const handleCancelConflict = () => {
    handleBackToCollect();
  };

  // ── 账户卡：返回重新获取连接信息 ──
  const handleBackToCollect = () => {
    if (loading) return;
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
    idConflict,
    overwriteApproved,
    confirmingOverwrite,
    handleOverwriteRecovery,
    handleRequestOverwrite,
    handleCancelConflict,
    handleCancelOverwriteConfirm,
  };
}
