import { useEffect, useRef, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useNavigate } from 'react-router-dom';
import { MIN_PASSWORD_LENGTH } from '@/lib/constants';
import { translateRustError } from '@/lib/rustErrors';
import { useAuthStore } from '@/stores/authStore';
import { useCameraCapability } from '@/hooks/useCameraCapability';
import type { AccountInfo } from '@/lib/ipc';
import type {
  RecoveryResultSummary,
  RecoveryDiscoveredHost,
  ScannedRecoveryQr,
  TabMode,
  Step,
} from '@/components/recovery/recoveryReceiveTypes';
import { MDNS_DISCOVER_TIMEOUT_MS, PIN_REGEX } from '@/components/recovery/recoveryReceiveTypes';

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

  // 已收集的连接信息（扫码或手动输入后统一进入账户卡阶段）
  const [pending, setPending] = useState<ScannedRecoveryQr | null>(null);
  const [masterPassword, setMasterPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordHint, setPasswordHint] = useState('');
  // 账户 ID 冲突（本设备已存在相同 account_id）→ 展示覆盖恢复选项
  const [idConflict, setIdConflict] = useState(false);
  // 用户已确认覆盖恢复 → 展示密码输入（覆盖模式），开始恢复时携带 overwrite=true
  const [overwriteApproved, setOverwriteApproved] = useState(false);
  // 二次确认覆盖弹窗是否打开
  const [confirmingOverwrite, setConfirmingOverwrite] = useState(false);
  // 校验错误（按优先级：主密码 > 确认密码；提示词可选不校验）
  const [masterPasswordError, setMasterPasswordError] = useState<string | null>(null);
  const [confirmPasswordError, setConfirmPasswordError] = useState<string | null>(null);

  // 手动连接表单状态
  const [hostAddr, setHostAddr] = useState('');
  const [pin, setPin] = useState('');
  const [fingerprint, setFingerprint] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [statusText, setStatusText] = useState<string | null>(null);

  // LAN discovery state
  const [scanning, setScanning] = useState(false);
  const [discoveredHosts, setDiscoveredHosts] = useState<RecoveryDiscoveredHost[]>([]);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scanDone, setScanDone] = useState(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // ── 扫描完成后的账户 ID 冲突预检：本设备已存在相同 account_id → 输入密码前即提示覆盖选项 ──
  const checkIdConflict = useCallback(async (accountId?: string | null): Promise<boolean> => {
    if (!accountId) return false;
    try {
      const accounts = await invoke<AccountInfo[]>('vault_list_accounts');
      return accounts.some((a) => a.id === accountId);
    } catch {
      // 检查失败视为无冲突，由后端在恢复时兜底提示
      return false;
    }
  }, []);

  // ── 连接失败：把底层错误映射为带诊断引导的友好提示 ──
  const friendlyConnectError = useCallback(
    (raw: string): string => {
      const lower = raw.toLowerCase();
      if (lower.includes('timed out') || lower.includes('timeout')) {
        // 连接超时：最常见原因是 macOS/Windows 防火墙拦截入站连接，
        // 或两台设备不在同一网络。给出可操作的排查步骤。
        return t('common:recovery_connect_timeout');
      }
      if (lower.includes('unreachable') || lower.includes('no route')) {
        // 目标网络不可达：典型为跨网段/不在同一网络。
        return t('common:recovery_connect_unreachable');
      }
      if (lower.includes('refused') || lower.includes('connection reset')) {
        // 连接被拒：主机端未在监听（会话过期/已取消）或端口不可达。
        return t('common:recovery_connect_refused');
      }
      if (
        lower.includes('read prefix failed') ||
        lower.includes('failed to fill whole buffer') ||
        lower.includes('unexpected eof') ||
        lower.includes('invalid magic prefix')
      ) {
        // 主机在发送任何数据前就关闭了连接（EOF）：会话可能已结束（超时/被取消/已被使用），
        // 或主机在握手前发生了内部错误。引导用户在旧设备上重新生成恢复二维码后重试。
        return t('common:recovery_host_closed_early');
      }
      if (lower.includes('too many failed recovery attempts')) {
        // 全局限流：短时间内失败次数过多，恢复服务暂时拒绝新连接。
        return t('common:recovery_too_many_attempts');
      }
      if (lower.includes('invalid pin') || lower.includes('invalid nonce')) {
        // PIN/二维码随机数不匹配：二维码可能已过期、已被使用，或 PIN 输入有误。
        return t('common:recovery_invalid_pin');
      }
      if (lower.includes('identity verification failed') || lower.includes('possible mitm')) {
        // 指纹校验失败：可能为中间人攻击，或指纹输入与旧设备屏幕不一致。
        return t('common:recovery_mitm');
      }
      if (
        lower.includes('read handshake') ||
        lower.includes('unexpected auth response') ||
        lower.includes('did not provide a static public key')
      ) {
        // 握手中断 / 协议异常：会话可能已中断或失效。
        return t('common:recovery_handshake_failed');
      }
      if (lower.includes('incomplete transfer') || lower.includes('received more data than expected')) {
        // 传输中断：连接在传输过程中断开。
        return t('common:recovery_transfer_failed');
      }
      if (lower.includes('export file too large') || lower.includes('invalid file size')) {
        // 恢复包超过大小限制。
        return t('common:recovery_package_too_large');
      }
      if (lower.includes('recovery task failed')) {
        // spawn_blocking join 失败（任务 panic/abort 时的内部错误）。
        return t('common:recovery_task_failed');
      }
      // 兜底：未命中的已知 Rust 错误先尝试 i18n 映射（如 Account ID already exists），
      // 命中则返回本地化文案，未命中才返回原始错误。
      const translated = translateRustError(raw);
      if (translated) return t(translated);
      return raw;
    },
    [t],
  );

  // ── 重置所有状态 ──
  const resetState = () => {
    setStep('collect');
    setTab(getDefaultTab());
    userSwitchedTabRef.current = false;
    setError(null);
    setSuccess(null);
    setScannerError(null);
    setLoading(false);
    setPending(null);
    setMasterPassword('');
    setConfirmPassword('');
    setPasswordHint('');
    setMasterPasswordError(null);
    setConfirmPasswordError(null);
    setIdConflict(false);
    setOverwriteApproved(false);
    setConfirmingOverwrite(false);
    setHostAddr('');
    setPin('');
    setFingerprint('');
    setShowAdvanced(false);
    setStatusText(null);
    setDiscoveredHosts([]);
    setScanError(null);
    setScanDone(false);
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
      const conflict = await checkIdConflict(parsed.u);
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

  // ── 局域网扫描 ──
  const handleScanLan = async () => {
    if (scanning) return;
    setScanning(true);
    setScanError(null);
    setDiscoveredHosts([]);
    setScanDone(false);

    try {
      const hosts = await invoke<RecoveryDiscoveredHost[]>('recovery_discover_hosts', {
        timeoutMs: MDNS_DISCOVER_TIMEOUT_MS,
      });
      if (!mountedRef.current) return;
      setDiscoveredHosts(hosts);
      setScanDone(true);
      if (hosts.length === 0) {
        setScanError(
          t('common:recovery_scan_no_hosts', { defaultValue: 'No recovery hosts found on the network.' })
        );
      }
    } catch (err) {
      if (!mountedRef.current) return;
      setScanError(String(err));
    } finally {
      setScanning(false);
    }
  };

  const handleSelectHost = (host: RecoveryDiscoveredHost) => {
    // 安全约束（P001）：mDNS TXT 不再广播 PIN/nonce，仅提供地址与指纹。
    // 选中主机后地址/指纹自动填充，PIN 需由用户在主机屏幕/QR 上查看后手动输入。
    setHostAddr(host.addr);
    setPin('');
    setFingerprint(host.fingerprint);
    setDiscoveredHosts([]);
    setScanDone(false);
    setScanError(null);
  };

  // ── 密码输入变更：清空对应校验错误与全局错误 ──
  // 注意：覆盖模式（overwriteApproved）下不清除 idConflict——密码输入框仅在确认覆盖后渲染，
  // 若在此清冲突会误切换到普通分支，导致 UI 显示与 overwrite=true 实际行为不一致。
  const handleMasterPasswordChange = (v: string) => {
    setMasterPassword(v);
    setError(null);
    if (idConflict && !overwriteApproved) setIdConflict(false);
    if (masterPasswordError) setMasterPasswordError(null);
  };

  const handleConfirmPasswordChange = (v: string) => {
    setConfirmPassword(v);
    setError(null);
    if (idConflict && !overwriteApproved) setIdConflict(false);
    if (confirmPasswordError) setConfirmPasswordError(null);
  };

  // ── 手动输入完成：校验后进入账户卡 ──
  const handleManualNext = () => {
    setError(null);
    if (!hostAddr.trim()) {
      setError(t('common:recovery_receive_addr_required', { defaultValue: 'Host address is required' }));
      return;
    }
    if (!PIN_REGEX.test(pin.trim())) {
      setError(t('common:recovery_receive_invalid_pin', { defaultValue: 'PIN must be a 6-digit code' }));
      return;
    }
    setPending({
      addr: hostAddr.trim(),
      pin: pin.trim(),
      fingerprint: fingerprint.trim(),
      nonce: null, // 手动模式不传 nonce，服务端兼容处理
    });
    setStep('account');
  };

  // ── 账户卡：设置主密码后开始恢复（与扫码路径走同一命令） ──
  const handleStartRecovery = async () => {
    if (!pending) return;
    setError(null);
    setSuccess(null);

    // 校验优先级（与创建账户页一致）：主密码未输入 > 主密码不符合要求 > 确认密码未输入 > 两次密码不一致；
    // 密码提示词为可选字段不校验。长度不足/不一致设置对应输入框 error，触发抖动+红边+红字。
    if (!masterPassword) {
      setMasterPasswordError(t('common:master_password_required'));
      return;
    }
    if (masterPassword.length < MIN_PASSWORD_LENGTH) {
      setMasterPasswordError(t('common:password_length_requirement'));
      return;
    }
    if (!confirmPassword) {
      setConfirmPasswordError(t('common:confirm_password_required'));
      return;
    }
    if (masterPassword !== confirmPassword) {
      setConfirmPasswordError(t('common:password_mismatch'));
      return;
    }

    setLoading(true);
    setStatusText(t('common:recovery_connecting', { defaultValue: 'Connecting to host…' }));

    try {
      const result = await invoke<RecoveryResultSummary>('recovery_restore_from_host', {
        hostAddr: pending.addr,
        pin: pending.pin,
        masterPassword,
        passwordHint: passwordHint.trim() || null,
        fingerprint: pending.fingerprint || null,
        nonce: pending.nonce,
        overwrite: overwriteApproved,
      });
      if (!mountedRef.current) return;
      setSuccess(result);
      setStep('success');
      await useAuthStore.getState().checkHasAccount();
    } catch (err) {
      if (!mountedRef.current) return;
      const raw = String(err);
      if (raw.includes('Account ID already exists')) {
        // 兜底（手动输入等无 accountId 预检的路径）：进入冲突状态，展示覆盖恢复选项（不显示普通错误）
        setIdConflict(true);
        setOverwriteApproved(false);
      } else {
        setError(friendlyConnectError(raw));
      }
    } finally {
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
    setMasterPassword('');
    setConfirmPassword('');
    setPasswordHint('');
    setMasterPasswordError(null);
    setConfirmPasswordError(null);
    setIdConflict(false);
    setOverwriteApproved(false);
    setConfirmingOverwrite(false);
    setError(null);
    setStep('collect');
  };

  return {
    step,
    tab,
    cameraCapability,
    loading,
    error,
    success,
    scannerError,
    pending,
    masterPassword,
    confirmPassword,
    passwordHint,
    masterPasswordError,
    confirmPasswordError,
    hostAddr,
    pin,
    fingerprint,
    showAdvanced,
    statusText,
    scanning,
    discoveredHosts,
    scanError,
    scanDone,
    setPasswordHint,
    setHostAddr,
    setPin,
    setFingerprint,
    setShowAdvanced,
    setScannerError,
    handleMasterPasswordChange,
    handleConfirmPasswordChange,
    handleClose,
    switchTab,
    handleScan,
    handleScanLan,
    handleSelectHost,
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
