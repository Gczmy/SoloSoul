import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { X, QrCode, Link2, Loader2, Wifi } from 'lucide-react';
import { QRCodeSVG } from 'qrcode.react';
import { useNavigate } from 'react-router-dom';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';

interface RecoveryReceiveDialogProps {
  isOpen: boolean;
  onClose: () => void;
  /** 恢复成功后调用；若提供则替代默认的 /home 导航 */
  onSuccess?: () => void;
}

interface RecoveryResultSummary {
  objectCount: number;
  attachmentCount: number;
}

interface ReverseListenInfo {
  displayAddr: string;
  pin: string;
  qrPayload: string;
}

interface RecoveryDiscoveredHost {
  name: string;
  addr: string;
  pin: string;
  fingerprint: string;
  nonce: string;
}

type TabMode = 'manual' | 'reverse';

const PIN_REGEX = /^\d{6}$/;
const MDNS_DISCOVER_TIMEOUT_MS = 5000;

export function RecoveryReceiveDialog({ isOpen, onClose, onSuccess }: RecoveryReceiveDialogProps) {
  const { t } = useTranslation(['common']);
  const navigate = useNavigate();
  const mountedRef = useRef(true);

  // Tab state
  const [tab, setTab] = useState<TabMode>('manual');

  // Shared state
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<RecoveryResultSummary | null>(null);

  // Manual connect form state
  const [hostAddr, setHostAddr] = useState('');
  const [pin, setPin] = useState('');
  const [fingerprint, setFingerprint] = useState('');
  const [masterPassword, setMasterPassword] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [statusText, setStatusText] = useState<string | null>(null);

  // LAN discovery state
  const [scanning, setScanning] = useState(false);
  const [discoveredHosts, setDiscoveredHosts] = useState<RecoveryDiscoveredHost[]>([]);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scanDone, setScanDone] = useState(false);

  // Reverse listen state
  const [reverseListenPassword, setReverseListenPassword] = useState('');
  const [reverseInfo, setReverseInfo] = useState<ReverseListenInfo | null>(null);
  const [reverseSession, setReverseSession] = useState<{
    info: ReverseListenInfo;
    password: string;
  } | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // 二维码显示后立即在后台等待传输；避免把长轮询阻塞在按钮事件里导致 UI 不更新
  useEffect(() => {
    if (!reverseSession) return;
    let active = true;
    const wait = async () => {
      try {
        const result = await invoke<RecoveryResultSummary>('recovery_receive_listen_wait', {
          masterPassword: reverseSession.password,
        });
        if (!active || !mountedRef.current) return;
        setSuccess(result);
        setReverseSession(null);
        await useAuthStore.getState().checkHasAccount();
      } catch (err) {
        if (!active || !mountedRef.current) return;
        const msg = String(err);
        if (msg.includes('Recovery session cancelled')) return;
        setError(msg);
        setReverseInfo(null);
        setReverseSession(null);
        setLoading(false);
      }
    };
    wait();
    return () => {
      active = false;
      // 组件卸载时清理反向聆听监听线程
      invoke('recovery_host_cancel').catch(() => {});
    };
  }, [reverseSession]);

  if (!isOpen) return null;

  // ── 重置所有状态 ──
  const resetState = () => {
    setError(null);
    setSuccess(null);
    setLoading(false);
    setHostAddr('');
    setPin('');
    setFingerprint('');
    setMasterPassword('');
    setShowAdvanced(false);
    setStatusText(null);
    setReverseListenPassword('');
  };

  const handleClose = () => {
    if (reverseInfo || reverseSession) {
      invoke('recovery_host_cancel').catch(() => {});
    }
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
    // 清理反向聆听状态
    if (reverseInfo || reverseSession) {
      invoke('recovery_host_cancel').catch(() => {});
    }
    resetState();
    setTab(newTab);
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

  // ── 手动连接恢复 ──
  const handleManualRecovery = async () => {
    setError(null);
    setSuccess(null);

    // 校验输入
    if (!hostAddr.trim()) {
      setError(t('common:recovery_receive_addr_required', { defaultValue: 'Host address is required' }));
      return;
    }
    if (!PIN_REGEX.test(pin.trim())) {
      setError(t('common:recovery_receive_invalid_pin', { defaultValue: 'PIN must be a 6-digit code' }));
      return;
    }
    if (masterPassword.length < 8) {
      setError(t('common:password_length_requirement'));
      return;
    }

    setLoading(true);
    setStatusText(t('common:recovery_connecting', { defaultValue: 'Connecting to host…' }));

    try {
      const result = await invoke<RecoveryResultSummary>('recovery_restore_from_host', {
        hostAddr: hostAddr.trim(),
        pin: pin.trim(),
        masterPassword,
        fingerprint: fingerprint.trim() || null,
        nonce: null, // 手动模式不传 nonce，服务端兼容处理
      });
      if (!mountedRef.current) return;
      setSuccess(result);
      await useAuthStore.getState().checkHasAccount();
    } catch (err) {
      if (!mountedRef.current) return;
      setError(String(err));
    } finally {
      setLoading(false);
      setStatusText(null);
    }
  };

  // ── 反向聆听（显示二维码） ──
  const handleStartReverseListen = async () => {
    setError(null);
    setSuccess(null);

    if (reverseListenPassword.length < 8) {
      setError(t('common:password_length_requirement'));
      return;
    }

    setLoading(true);
    try {
      const info = await invoke<ReverseListenInfo>('recovery_receive_listen_start');
      if (!mountedRef.current) return;
      setReverseInfo(info);
      setReverseSession({ info, password: reverseListenPassword });
    } catch (err) {
      if (!mountedRef.current) return;
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleCancelReverse = () => {
    invoke('recovery_host_cancel').catch(() => {});
    setReverseInfo(null);
    setReverseSession(null);
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

        {/* Tab 切换 */}
        {!success && !reverseInfo && (
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
            <button
              type="button"
              onClick={() => switchTab('reverse')}
              style={{
                flex: 1,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 6,
                padding: '8px 12px',
                borderRadius: 8,
                border: 'none',
                background: tab === 'reverse' ? 'var(--bg-elevated)' : 'transparent',
                color: tab === 'reverse' ? 'var(--accent-primary)' : 'var(--text-tertiary)',
                cursor: loading ? 'not-allowed' : 'pointer',
                fontFamily: 'inherit',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                transition: 'all 0.15s ease',
                opacity: loading ? 0.5 : 1,
              }}
            >
              <QrCode size={16} />
              {t('common:recovery_qr_show_button')}
            </button>
          </div>
        )}

        {success ? (
          /* ── 成功 ── */
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
              <span style={{ fontSize: 28 }}>🎉</span>
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
        ) : reverseInfo ? (
          /* ── 反向聆听（已显示二维码） ── */
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'center' }}>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: 0,
                lineHeight: 1.5,
                textAlign: 'center',
              }}
            >
              {t('common:recovery_receive_reverse_desc', {
                defaultValue: 'Show this QR code to your other device, then scan it to push your data here.'
              })}
            </p>
            <div
              style={{
                padding: 12,
                background: '#fff',
                borderRadius: 12,
                border: '1px solid var(--border-subtle)',
              }}
            >
              <QRCodeSVG value={reverseInfo.qrPayload} size={200} level="M" includeMargin />
            </div>

            <div style={{ width: '100%' }}>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '10px 12px',
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  marginBottom: 8,
                }}
              >
                <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                  {t('common:recovery_host_pin_label')}
                </span>
                <span
                  style={{
                    fontFamily: 'monospace',
                    fontSize: 'var(--text-body)',
                    fontWeight: 700,
                    letterSpacing: 4,
                    color: 'var(--accent-primary)',
                  }}
                >
                  {reverseInfo.pin}
                </span>
              </div>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '10px 12px',
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
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
                  {reverseInfo.displayAddr}
                </span>
              </div>
            </div>

            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '8px 12px',
                borderRadius: 8,
                background: 'rgba(241,196,15,0.08)',
                border: '1px solid rgba(241,196,15,0.2)',
                width: '100%',
                boxSizing: 'border-box',
              }}
            >
              <Loader2 size={14} style={{ animation: 'spin 1s linear infinite', flexShrink: 0 }} />
              <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-secondary)' }}>
                {t('common:recovery_host_waiting')}
              </span>
            </div>

            <Button variant="secondary" onClick={handleCancelReverse} style={{ width: '100%' }}>
              {t('common:recovery_host_cancel')}
            </Button>
          </div>
        ) : tab === 'manual' ? (
          /* ── 手动连接表单 ── */
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

            <Input
              label={t('common:recovery_receive_password_label')}
              type="password"
              value={masterPassword}
              onChange={(e) => setMasterPassword(e.target.value)}
              placeholder={t('common:recovery_receive_password_hint')}
              disabled={loading}
            />

            <Button
              onClick={handleManualRecovery}
              disabled={loading}
              loading={loading}
              style={{ width: '100%', marginTop: 4 }}
            >
              {loading
                ? (statusText || t('common:loading'))
                : t('common:recovery_receive_start')}
            </Button>

            {error && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
            )}
          </div>
        ) : (
          /* ── 反向聆听初始界面（二维码） ── */
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: '0 0 4px',
                lineHeight: 1.5,
              }}
            >
              {t('common:recovery_receive_reverse_initial_desc', {
                defaultValue: 'Set a new master password for this device, then show the QR code to your other device to scan.'
              })}
            </p>

            <Input
              label={t('common:recovery_receive_password_label')}
              type="password"
              value={reverseListenPassword}
              onChange={(e) => setReverseListenPassword(e.target.value)}
              placeholder={t('common:recovery_receive_password_hint')}
              autoFocus
            />

            <Button
              onClick={handleStartReverseListen}
              disabled={loading}
              loading={loading}
              style={{ width: '100%' }}
            >
              {loading
                ? t('common:loading')
                : t('common:recovery_qr_show_button')}
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
