import { useEffect, useRef, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { X, QrCode, Link2, Loader2, Wifi, CheckCircle2, CameraOff } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';
import { useCameraCapability } from '@/hooks/useCameraCapability';
import { RecoveryQrScanner } from '@/components/recovery/RecoveryQrScanner';

interface RecoveryReceiveDialogProps {
  isOpen: boolean;
  onClose: () => void;
  /** 恢复成功后调用；若提供则替代默认的 /home 导航 */
  onSuccess?: () => void;
}

interface RecoveryResultSummary {
  objectCount: number;
  attachmentCount: number;
  /** 恢复包的账户 ID（与旧设备一致）。 */
  accountId: string;
  /** 恢复包的账户名。 */
  accountName: string;
}

interface RecoveryDiscoveredHost {
  name: string;
  addr: string;
  pin: string;
  fingerprint: string;
  nonce: string;
}

/** 从 `t:"rec"` 二维码解析出的恢复连接信息。 */
interface ScannedRecoveryQr {
  addr: string;
  pin: string;
  fingerprint: string;
  nonce: string | null;
  /** 二维码中携带的账户 ID（预览用，最终以后端回传为准）。 */
  accountId?: string;
  /** 二维码中携带的账户名（预览用，最终以后端回传为准）。 */
  accountName?: string;
}

type TabMode = 'scan' | 'manual';
/** 流程阶段：collect=获取连接信息（扫码/手动），account=账户卡+设置主密码，success=成功卡片 */
type Step = 'collect' | 'account' | 'success';

const PIN_REGEX = /^\d{6}$/;
const MDNS_DISCOVER_TIMEOUT_MS = 5000;

export function RecoveryReceiveDialog({ isOpen, onClose, onSuccess }: RecoveryReceiveDialogProps) {
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

  if (!isOpen) return null;

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

  // ── 扫码：解析 t:"rec" 二维码 → 进入账户卡 ──
  const handleScan = (text: string) => {
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
    setHostAddr(host.addr);
    setPin(host.pin);
    setFingerprint(host.fingerprint);
    setDiscoveredHosts([]);
    setScanDone(false);
    setScanError(null);
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

    if (masterPassword.length < 8) {
      setError(t('common:password_length_requirement'));
      return;
    }

    setLoading(true);
    setStatusText(t('common:recovery_connecting', { defaultValue: 'Connecting to host…' }));

    try {
      const result = await invoke<RecoveryResultSummary>('recovery_restore_from_host', {
        hostAddr: pending.addr,
        pin: pending.pin,
        masterPassword,
        fingerprint: pending.fingerprint || null,
        nonce: pending.nonce,
      });
      if (!mountedRef.current) return;
      setSuccess(result);
      setStep('success');
      await useAuthStore.getState().checkHasAccount();
    } catch (err) {
      if (!mountedRef.current) return;
      setError(String(err));
    } finally {
      setLoading(false);
      setStatusText(null);
    }
  };

  // ── 账户卡：返回重新获取连接信息 ──
  const handleBackToCollect = () => {
    if (loading) return;
    setPending(null);
    setMasterPassword('');
    setError(null);
    setStep('collect');
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
        padding: 16,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <Card
        style={{
          maxWidth: 420,
          width: '100%',
          padding: 24,
          position: 'relative',
        }}
      >
        <button
          type="button"
          onClick={handleClose}
          style={{
            position: 'absolute',
            top: 12,
            right: 12,
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--text-tertiary)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
          aria-label={t('common:close')}
        >
          <X size={20} />
        </button>

        <h2
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 700,
            margin: '0 0 8px',
            color: 'var(--text-primary)',
            paddingRight: 24,
          }}
        >
          {t('common:recovery_receive_title')}
        </h2>

        {/* Tab 切换（仅收集连接信息阶段显示） */}
        {step === 'collect' && (
          <div
            style={{
              display: 'flex',
              gap: 4,
              marginBottom: 16,
              background: 'var(--bg-toolbar)',
              borderRadius: 10,
              padding: 3,
            }}
          >
            <button
              type="button"
              onClick={() => switchTab('scan')}
              style={{
                flex: 1,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 6,
                padding: '8px 12px',
                borderRadius: 8,
                border: 'none',
                background: tab === 'scan' ? 'var(--bg-elevated)' : 'transparent',
                color: tab === 'scan' ? 'var(--accent-primary)' : 'var(--text-tertiary)',
                cursor: loading ? 'not-allowed' : 'pointer',
                fontFamily: 'inherit',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                transition: 'all 0.15s ease',
                opacity: loading ? 0.5 : 1,
              }}
            >
              <QrCode size={16} />
              {t('common:recovery_scan_tab', { defaultValue: 'Scan QR' })}
            </button>
            <button
              type="button"
              onClick={() => switchTab('manual')}
              style={{
                flex: 1,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 6,
                padding: '8px 12px',
                borderRadius: 8,
                border: 'none',
                background: tab === 'manual' ? 'var(--bg-elevated)' : 'transparent',
                color: tab === 'manual' ? 'var(--accent-primary)' : 'var(--text-tertiary)',
                cursor: loading ? 'not-allowed' : 'pointer',
                fontFamily: 'inherit',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                transition: 'all 0.15s ease',
                opacity: loading ? 0.5 : 1,
              }}
            >
              <Link2 size={16} />
              {t('common:recovery_manual_tab', { defaultValue: 'Manual' })}
            </button>
          </div>
        )}

        {step === 'success' && success ? (
          /* ── 成功卡片：显示账户信息 + 导入统计 ── */
          <div style={{ textAlign: 'center', padding: '12px 0' }}>
            <div
              style={{
                width: 56,
                height: 56,
                borderRadius: '50%',
                background: 'rgba(39,174,96,0.12)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                margin: '0 auto 16px',
              }}
            >
              <CheckCircle2 size={32} color="#27ae60" />
            </div>
            <h3
              style={{
                fontSize: 'var(--text-body)',
                fontWeight: 600,
                margin: '0 0 8px',
                color: 'var(--text-primary)',
              }}
            >
              {t('common:recovery_receive_success')}
            </h3>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 8,
                textAlign: 'left',
                margin: '16px 0',
                padding: '12px 14px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                }}
              >
                <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                  {t('common:recovery_account_name_label', { defaultValue: 'Account Name' })}
                </span>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                  {success.accountName}
                </span>
              </div>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                }}
              >
                <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                  {t('common:recovery_account_id_label', { defaultValue: 'Account ID' })}
                </span>
                <span
                  style={{
                    fontFamily: 'monospace',
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-primary)',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    maxWidth: '60%',
                  }}
                >
                  {success.accountId}
                </span>
              </div>
            </div>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: '0 0 24px',
              }}
            >
              {t('common:recovery_receive_success_desc', {
                objects: success.objectCount,
                attachments: success.attachmentCount,
              })}
            </p>
            <Button onClick={handleClose} style={{ width: '100%' }}>
              {t('common:onboarding_done')}
            </Button>
          </div>
        ) : step === 'account' && pending ? (
          /* ── 账户卡：确认账户/连接信息 + 设置主密码（连接前） ── */
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: '0 0 4px',
                lineHeight: 1.5,
              }}
            >
              {pending.accountName
                ? t('common:recovery_account_card_desc_scan', {
                    defaultValue: 'Account detected. Set a new master password for this device, then start recovery.',
                  })
                : t('common:recovery_account_card_desc_manual', {
                    defaultValue: 'Connection details ready. Set a new master password for this device, then start recovery.',
                  })}
            </p>

            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 8,
                padding: '12px 14px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
              }}
            >
              {pending.accountName && (
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                  }}
                >
                  <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                    {t('common:recovery_account_name_label', { defaultValue: 'Account Name' })}
                  </span>
                  <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                    {pending.accountName}
                  </span>
                </div>
              )}
              {pending.accountId && (
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                  }}
                >
                  <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                    {t('common:recovery_account_id_label', { defaultValue: 'Account ID' })}
                  </span>
                  <span
                    style={{
                      fontFamily: 'monospace',
                      fontSize: 'var(--text-body-sm)',
                      color: 'var(--text-primary)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      maxWidth: '60%',
                    }}
                  >
                    {pending.accountId}
                  </span>
                </div>
              )}
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                }}
              >
                <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                  {t('common:recovery_host_addr_label')}
                </span>
                <span
                  style={{
                    fontFamily: 'monospace',
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-primary)',
                  }}
                >
                  {pending.addr}
                </span>
              </div>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                }}
              >
                <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                  {t('common:recovery_host_pin_label')}
                </span>
                <span
                  style={{
                    fontFamily: 'monospace',
                    fontSize: 'var(--text-body-sm)',
                    fontWeight: 700,
                    letterSpacing: 4,
                    color: 'var(--accent-primary)',
                  }}
                >
                  {pending.pin}
                </span>
              </div>
            </div>

            {/* 传输中的状态提示 */}
            {loading && statusText && (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '10px 12px',
                  borderRadius: 8,
                  background: 'rgba(52,152,219,0.08)',
                  border: '1px solid rgba(52,152,219,0.2)',
                  width: '100%',
                  boxSizing: 'border-box',
                }}
              >
                <Loader2
                  size={16}
                  style={{ animation: 'spin 1s linear infinite', flexShrink: 0 }}
                />
                <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                  {statusText}
                </span>
              </div>
            )}

            <Input
              label={t('common:recovery_receive_password_label')}
              type="password"
              value={masterPassword}
              onChange={(e) => setMasterPassword(e.target.value)}
              placeholder={t('common:recovery_receive_password_hint')}
              disabled={loading}
              autoFocus
            />

            <Button
              onClick={handleStartRecovery}
              disabled={loading}
              loading={loading}
              style={{ width: '100%', marginTop: 4 }}
            >
              {loading
                ? (statusText || t('common:loading'))
                : t('common:recovery_receive_start')}
            </Button>

            <Button
              variant="secondary"
              onClick={handleBackToCollect}
              disabled={loading}
              style={{ width: '100%' }}
            >
              {t('common:back')}
            </Button>

            {error && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
            )}
          </div>
        ) : tab === 'scan' ? (
          /* ── 扫码 tab：新设备摄像头扫描旧设备恢复二维码 ── */
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: '0 0 4px',
                lineHeight: 1.5,
              }}
            >
              {t('common:recovery_receive_scan_desc', {
                defaultValue:
                  'On your old device, go to Settings → Device Sync → Show Recovery QR, then scan it with this camera.',
              })}
            </p>

            {cameraCapability === 'unsupported' ? (
              /* 设备无摄像头：扫码位置显示提示，引导使用手动输入 */
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 10,
                  padding: '28px 16px',
                  borderRadius: 12,
                  border: '1px dashed var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  textAlign: 'center',
                }}
              >
                <CameraOff size={28} color="var(--text-tertiary)" />
                <span
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.5,
                  }}
                >
                  {t('common:recovery_scan_unsupported', {
                    defaultValue: 'This device does not support QR scanning. Please use manual input mode.',
                  })}
                </span>
                <button
                  type="button"
                  onClick={() => switchTab('manual')}
                  style={{
                    marginTop: 4,
                    padding: '8px 16px',
                    borderRadius: 8,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-elevated)',
                    color: 'var(--accent-primary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontSize: 'var(--text-body-sm)',
                    fontWeight: 500,
                    transition: 'all 0.15s ease',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }}
                >
                  {t('common:recovery_manual_tab', { defaultValue: 'Manual' })}
                </button>
              </div>
            ) : (
              <RecoveryQrScanner onScan={handleScan} onError={setScannerError} />
            )}

            {/* 扫码启动失败（权限被拒/无摄像头）时，提供手动输入的兜底入口 */}
            {scannerError && cameraCapability !== 'unsupported' && (
              <button
                type="button"
                onClick={() => switchTab('manual')}
                style={{
                  padding: '10px 12px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  color: 'var(--accent-primary)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  fontSize: 'var(--text-body-sm)',
                  fontWeight: 500,
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                }}
              >
                {t('common:recovery_use_manual', {
                  defaultValue: 'Use manual input mode',
                })}
              </button>
            )}

            {error && (
              <div
                style={{
                  color: '#e74c3c',
                  fontSize: 'var(--text-body-sm)',
                  textAlign: 'center',
                }}
              >
                {error}
              </div>
            )}
          </div>
        ) : (
          /* ── 手动输入 tab：无摄像头设备兜底 ── */
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: '0 0 4px',
                lineHeight: 1.5,
              }}
            >
              {t('common:recovery_receive_desc')}
            </p>

            {/* ── 局域网扫描 ── */}
            <div
              style={{
                padding: '10px 12px',
                borderRadius: 8,
                border: '1px dashed var(--border-subtle)',
                marginBottom: 8,
              }}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  marginBottom: discoveredHosts.length > 0 || scanError ? 8 : 0,
                }}
              >
                <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                  {t('common:recovery_scan_lan_label', { defaultValue: 'LAN Discovery' })}
                </span>
                <button
                  type="button"
                  onClick={handleScanLan}
                  disabled={scanning || loading}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    padding: '5px 10px',
                    borderRadius: 6,
                    border: scanning
                      ? '1px solid var(--border-subtle)'
                      : '1px solid transparent',
                    background: scanning
                      ? 'var(--bg-toolbar)'
                      : 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                    color: scanning ? 'var(--text-tertiary)' : 'var(--accent-primary)',
                    cursor: scanning || loading ? 'not-allowed' : 'pointer',
                    fontFamily: 'inherit',
                    fontSize: 'var(--text-caption)',
                    fontWeight: 500,
                    transition: 'all 0.15s ease',
                    opacity: scanning || loading ? 0.6 : 1,
                  }}
                  onMouseEnter={(e) => {
                    if (scanning || loading) return;
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 14%, transparent)';
                  }}
                  onMouseLeave={(e) => {
                    if (scanning || loading) return;
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 8%, transparent)';
                  }}
                >
                  {scanning ? (
                    <Loader2 size={14} style={{ animation: 'spin 1s linear infinite' }} />
                  ) : (
                    <Wifi size={14} />
                  )}
                  {scanning
                    ? t('common:recovery_scan_scanning', { defaultValue: 'Scanning…' })
                    : t('common:recovery_scan_button', { defaultValue: 'Scan LAN' })}
                </button>
              </div>

              {/* 发现的设备列表 */}
              {discoveredHosts.length > 0 && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                  {discoveredHosts.map((host, i) => (
                    <button
                      key={`${host.addr}-${i}`}
                      type="button"
                      onClick={() => handleSelectHost(host)}
                      disabled={loading}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        padding: '8px 10px',
                        borderRadius: 6,
                        border: '1px solid var(--border-subtle)',
                        background: 'var(--bg-elevated)',
                        cursor: loading ? 'not-allowed' : 'pointer',
                        fontFamily: 'inherit',
                        textAlign: 'left',
                        transition: 'all 0.15s ease',
                        opacity: loading ? 0.6 : 1,
                      }}
                      onMouseEnter={(e) => {
                        if (!loading)
                          e.currentTarget.style.borderColor = 'var(--accent-primary)';
                      }}
                      onMouseLeave={(e) => {
                        if (!loading)
                          e.currentTarget.style.borderColor = 'var(--border-subtle)';
                      }}
                    >
                      <div style={{ minWidth: 0 }}>
                        <div
                          style={{
                            fontSize: 'var(--text-body-sm)',
                            fontWeight: 500,
                            color: 'var(--text-primary)',
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {host.name}
                        </div>
                        <div
                          style={{
                            fontSize: 'var(--text-caption)',
                            color: 'var(--text-tertiary)',
                            fontFamily: 'monospace',
                          }}
                        >
                          {host.addr}
                        </div>
                      </div>
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 4,
                          padding: '2px 6px',
                          borderRadius: 4,
                          background: 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                          color: 'var(--accent-primary)',
                          fontSize: 'var(--text-caption)',
                          fontFamily: 'monospace',
                          fontWeight: 600,
                          letterSpacing: 2,
                        }}
                      >
                        {host.pin}
                      </div>
                    </button>
                  ))}
                </div>
              )}

              {scanError && !scanning && (
                <div
                  style={{
                    fontSize: 'var(--text-caption)',
                    color: scanDone && discoveredHosts.length === 0
                      ? 'var(--text-tertiary)'
                      : '#e74c3c',
                    padding: '2px 0',
                  }}
                >
                  {scanError}
                </div>
              )}
            </div>

            <Input
              label={t('common:recovery_receive_addr_label')}
              type="text"
              value={hostAddr}
              onChange={(e) => setHostAddr(e.target.value)}
              placeholder={t('common:recovery_receive_addr_placeholder')}
              disabled={loading}
            />

            <Input
              label={t('common:recovery_receive_pin_label')}
              type="text"
              value={pin}
              onChange={(e) => setPin(e.target.value.replace(/\D/g, '').slice(0, 6))}
              placeholder="123456"
              maxLength={6}
              disabled={loading}
              style={{ fontFamily: 'monospace', letterSpacing: 4, fontSize: 'var(--text-body)' }}
            />

            {/* 展开/收起高级选项（指纹） */}
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              disabled={loading}
              style={{
                background: 'none',
                border: 'none',
                color: 'var(--text-tertiary)',
                fontSize: 'var(--text-caption)',
                cursor: loading ? 'not-allowed' : 'pointer',
                fontFamily: 'inherit',
                padding: '2px 0',
                textAlign: 'left',
                transition: 'color 0.15s ease',
              }}
              onMouseEnter={(e) => {
                if (!loading) e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                if (!loading) e.currentTarget.style.color = 'var(--text-tertiary)';
              }}
            >
              {showAdvanced
                ? t('common:recovery_advanced_hide', { defaultValue: 'Hide optional fingerprint' })
                : t('common:recovery_advanced_show', { defaultValue: 'Show optional fingerprint' })}
            </button>

            {showAdvanced && (
              <Input
                label={t('common:recovery_receive_fingerprint_label')}
                type="text"
                value={fingerprint}
                onChange={(e) => setFingerprint(e.target.value)}
                placeholder={t('common:recovery_fingerprint_placeholder', { defaultValue: 'e.g. abc123…' })}
                disabled={loading}
              />
            )}

            <Button
              onClick={handleManualNext}
              disabled={loading}
              style={{ width: '100%', marginTop: 4 }}
            >
              {t('common:next')}
            </Button>

            {error && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
            )}
          </div>
        )}
      </Card>
    </div>
  );
}
