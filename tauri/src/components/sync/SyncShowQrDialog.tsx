import { useEffect, useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { QRCodeSVG } from 'qrcode.react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { motion } from 'framer-motion';
import { X, Loader2, Copy, Check, ChevronDown, ChevronUp, QrCode, LifeBuoy } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { translateRustError } from '@/lib/rustErrors';

interface SyncShowQrDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

interface SyncQrInfo {
  payload: string;
  addr: string;
  fingerprint: string;
  deviceName: string;
}

interface RecoveryHostInfo {
  displayAddr: string;
  bindAddr: string;
  pin: string;
  qrPayload: string;
}

type QrMode = 'sync' | 'recovery';

export function SyncShowQrDialog({ isOpen, onClose }: SyncShowQrDialogProps) {
  const { t } = useTranslation(['common', 'settings']);

  // 当前显示的二维码类型
  const [mode, setMode] = useState<QrMode>('sync');

  // ── 同步二维码状态 ──
  const [info, setInfo] = useState<SyncQrInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ── 恢复二维码状态 ──
  const [recoveryInfo, setRecoveryInfo] = useState<RecoveryHostInfo | null>(null);
  const [recoveryLoading, setRecoveryLoading] = useState(false);
  const [recoveryError, setRecoveryError] = useState<string | null>(null);
  const [manualOpen, setManualOpen] = useState(false);
  const [copiedAddr, setCopiedAddr] = useState(false);
  const [copiedPin, setCopiedPin] = useState(false);
  // 防止重复启动 / 重复取消恢复会话（state 异步更新，用 ref 保证生命周期正确）
  const recoveryStartedRef = useRef(false);

  // 打开对话框：重置状态并加载同步二维码
  useEffect(() => {
    if (!isOpen) {
      // 关闭/重置：若恢复会话仍存活则取消（幂等，正常关闭路径 handleClose 已处理）
      if (recoveryStartedRef.current) {
        invoke('recovery_host_cancel').catch(() => {});
        recoveryStartedRef.current = false;
      }
      setInfo(null);
      setError(null);
      setRecoveryInfo(null);
      setRecoveryError(null);
      setManualOpen(false);
      setMode('sync');
      return;
    }

    const TIMEOUT_MS = 10_000;
    setLoading(true);

    // 超时保护：如果后端 10 秒无响应，自动取消 loading 并显示错误
    const timeoutId = setTimeout(() => {
      setError(
        t('common:sync_qr_timeout', {
          defaultValue: 'QR generation timed out. Please try again.',
        }),
      );
      setLoading(false);
    }, TIMEOUT_MS);

    invoke<string>('sync_generate_qr_payload')
      .then((payload) => {
        clearTimeout(timeoutId);
        try {
          const parsed = JSON.parse(payload);
          setInfo({
            payload,
            addr: parsed.a || '',
            fingerprint: parsed.f || '',
            deviceName: parsed.n || '',
          });
        } catch {
          setError(t('common:sync_qr_invalid_payload'));
        }
      })
      .catch((err) => {
        clearTimeout(timeoutId);
        // 后端错误码（如 __SYNC_ERR__:not_enabled）经 resolveBackendErrorMessage 国际化
        setError(resolveBackendErrorMessage(err));
      })
      .finally(() => {
        clearTimeout(timeoutId);
        setLoading(false);
      });
  }, [isOpen, t]);

  // 页面卸载时兜底取消恢复会话，避免导航离开同步页后会话悬挂至过期
  useEffect(() => {
    return () => {
      if (recoveryStartedRef.current) {
        invoke('recovery_host_cancel').catch(() => {});
        recoveryStartedRef.current = false;
      }
    };
  }, []);

  // 关闭对话框：若恢复会话已启动则先取消
  const handleClose = () => {
    if (recoveryStartedRef.current) {
      invoke('recovery_host_cancel').catch(() => {});
      recoveryStartedRef.current = false;
    }
    onClose();
  };

  // 启动恢复主机会话（首次切换到「恢复二维码」时自动调用）
  const startRecoveryHost = async () => {
    if (recoveryStartedRef.current) return;
    setRecoveryLoading(true);
    setRecoveryError(null);
    recoveryStartedRef.current = true;
    try {
      const result = await invoke<RecoveryHostInfo>('recovery_host_start');
      setRecoveryInfo(result);
    } catch (err) {
      // 后端错误码（如 __SYNC_ERR__:not_enabled）经 resolveBackendErrorMessage 国际化；
      // 静态 Rust 错误串（如 No account is currently unlocked）经 translateRustError 映射兜底。
      const raw = String(err);
      const translated = translateRustError(raw);
      setRecoveryError(translated ? t(translated) : resolveBackendErrorMessage(raw));
      recoveryStartedRef.current = false;
    } finally {
      setRecoveryLoading(false);
    }
  };

  // 取消恢复主机会话（切回「同步二维码」时调用）
  const cancelRecoveryHost = () => {
    if (!recoveryStartedRef.current) return;
    recoveryStartedRef.current = false;
    invoke('recovery_host_cancel').catch(() => {});
    setRecoveryInfo(null);
    setRecoveryError(null);
    setManualOpen(false);
  };

  // 切换二维码类型
  const switchMode = (next: QrMode) => {
    if (next === mode) return;
    if (next === 'recovery') {
      setMode('recovery');
      startRecoveryHost();
    } else {
      cancelRecoveryHost();
      setMode('sync');
    }
  };

  const copyToClipboard = async (text: string, setCopied: (v: boolean) => void) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // fallback for older browsers
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.style.position = 'fixed';
      textarea.style.opacity = '0';
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (!isOpen) return null;

  const isRecovery = mode === 'recovery';

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
      {/* 卡片进场淡入：消除手写模态的硬弹出闪烁（与共享 Dialog 的 dialogIn 动画对齐） */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.2 }}
        style={{ width: '100%', maxWidth: 420 }}
      >
        <Card
          style={{
            // 宽度由外层 motion.div（width:100% + maxWidth:420）约束，这里只填满，避免双重 maxWidth
            width: '100%',
            padding: 24,
            position: 'relative',
            // 与 Dialog 组件一致：展开「手动模式」后内容较高，超出视口时允许卡片内滚动，
            // 避免 flex 居中溢出导致上下内容（tab 切换/关闭/取消按钮）不可达。
            // 注意：必须用视口单位 100vh 而非百分比 100% —— 父级 motion.div 高度为 auto，
            // 百分比无法解析会使整个 min() 失效，导致 max-height 不生效、内容全高溢出。
            maxHeight: 'min(85vh, calc(100vh - 32px))',
            overflowY: 'auto',
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
            }}
            aria-label={t('common:close')}
          >
            <X size={20} />
          </button>

          {/* 二维码类型切换 */}
          <div
            style={{
              display: 'flex',
              gap: 4,
              padding: 4,
              borderRadius: 10,
              background: 'var(--bg-toolbar)',
              marginBottom: 16,
            }}
          >
            <button
              type="button"
              onClick={() => switchMode('sync')}
              style={{
                flex: 1,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 6,
                padding: '8px 0',
                borderRadius: 8,
                border: 'none',
                background: !isRecovery ? 'var(--bg-elevated)' : 'transparent',
                color: !isRecovery ? 'var(--accent-primary)' : 'var(--text-secondary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: 'pointer',
                fontFamily: 'inherit',
                transition: 'all 0.15s ease',
              }}
            >
              <QrCode size={16} />
              {t('settings:sync_qr_tab_sync', { defaultValue: 'Sync QR' })}
            </button>
            <button
              type="button"
              onClick={() => switchMode('recovery')}
              style={{
                flex: 1,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 6,
                padding: '8px 0',
                borderRadius: 8,
                border: 'none',
                background: isRecovery ? 'var(--bg-elevated)' : 'transparent',
                color: isRecovery ? 'var(--accent-primary)' : 'var(--text-secondary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: 'pointer',
                fontFamily: 'inherit',
                transition: 'all 0.15s ease',
              }}
            >
              <LifeBuoy size={16} />
              {t('settings:sync_qr_tab_recovery', { defaultValue: 'Recovery QR' })}
            </button>
          </div>

          {isRecovery ? (
            <>
              <h2
                style={{
                  fontSize: 'var(--text-card-title)',
                  fontWeight: 700,
                  margin: '0 0 8px',
                  color: 'var(--text-primary)',
                }}
              >
                {t('common:recovery_host_title')}
              </h2>
              <p
                style={{
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-secondary)',
                  margin: '0 0 20px',
                  lineHeight: 1.5,
                }}
              >
                {t('common:recovery_host_desc')}
              </p>

              {recoveryLoading ? (
                // 数据未就绪时渲染固定高度的加载占位，与内容区同高 → 卡片高度不再突变
                <div
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 12,
                    minHeight: 360,
                  }}
                >
                  <Loader2 size={32} style={{ animation: 'spin 1s linear infinite' }} />
                  <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                    {t('common:loading')}
                  </span>
                </div>
              ) : recoveryError ? (
                <div
                  style={{
                    color: '#e74c3c',
                    fontSize: 'var(--text-body-sm)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    minHeight: 360,
                  }}
                >
                  {recoveryError}
                </div>
              ) : recoveryInfo ? (
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ duration: 0.2 }}
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 16,
                    alignItems: 'center',
                    minHeight: 360,
                  }}
                >
                  {/* QR 码 */}
                  <div
                    style={{
                      padding: 12,
                      background: '#fff',
                      borderRadius: 12,
                      border: '1px solid var(--border-subtle)',
                    }}
                  >
                    <QRCodeSVG value={recoveryInfo.qrPayload} size={200} level="M" includeMargin />
                  </div>
                  <p
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-tertiary)',
                      margin: 0,
                      textAlign: 'center',
                    }}
                  >
                    {t('common:recovery_host_qr_hint', {
                      defaultValue: 'Scan with the other device to connect automatically',
                    })}
                  </p>

                  {/* 网络信息 */}
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
                      <span
                        style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}
                      >
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
                        {recoveryInfo.pin}
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
                      <span
                        style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}
                      >
                        {t('common:recovery_host_addr_label')}
                      </span>
                      <span
                        style={{
                          fontFamily: 'monospace',
                          fontSize: 'var(--text-body-sm)',
                          color: 'var(--text-primary)',
                        }}
                      >
                        {recoveryInfo.displayAddr}
                      </span>
                    </div>
                  </div>

                  {/* localhost 警告 */}
                  {/^(127\.|::1|\[::1\])/.test(recoveryInfo.displayAddr) && (
                    <p
                      style={{
                        fontSize: 'var(--text-caption)',
                        color: 'var(--warning)',
                        textAlign: 'center',
                        margin: 0,
                      }}
                    >
                      {t('common:recovery_host_localhost_warning')}
                    </p>
                  )}

                  {/* 手动输入指引 — 折叠面板 */}
                  <div style={{ width: '100%' }}>
                    <button
                      type="button"
                      onClick={() => setManualOpen(!manualOpen)}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        width: '100%',
                        padding: '8px 10px',
                        borderRadius: 8,
                        border: '1px solid var(--border-subtle)',
                        background: manualOpen
                          ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
                          : 'transparent',
                        color: 'var(--text-secondary)',
                        cursor: 'pointer',
                        fontFamily: 'inherit',
                        fontSize: 'var(--text-body-sm)',
                        transition: 'all 0.15s ease',
                      }}
                    >
                      <span style={{ fontWeight: 500 }}>
                        {manualOpen
                          ? t('common:recovery_host_manual_hide', {
                              defaultValue: 'Hide manual entry guide',
                            })
                          : t('common:recovery_host_manual_show', {
                              defaultValue: 'No camera? Enter details manually',
                            })}
                      </span>
                      {manualOpen ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                    </button>

                    {manualOpen && (
                      <div
                        style={{
                          marginTop: 10,
                          padding: '12px 14px',
                          borderRadius: 8,
                          background: 'var(--bg-toolbar)',
                          display: 'flex',
                          flexDirection: 'column',
                          gap: 10,
                        }}
                      >
                        <p
                          style={{
                            fontSize: 'var(--text-body-sm)',
                            color: 'var(--text-secondary)',
                            margin: 0,
                            lineHeight: 1.5,
                          }}
                        >
                          {t('common:recovery_host_manual_desc', {
                            defaultValue:
                              'On the other device, open "Restore from another device", choose the Manual tab, and enter:',
                          })}
                        </p>

                        {/* 可复制地址 */}
                        <div
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between',
                            padding: '8px 10px',
                            borderRadius: 6,
                            background: 'var(--bg-elevated)',
                            border: '1px solid var(--border-subtle)',
                          }}
                        >
                          <div style={{ minWidth: 0 }}>
                            <div
                              style={{
                                fontSize: 'var(--text-caption)',
                                color: 'var(--text-tertiary)',
                                marginBottom: 2,
                              }}
                            >
                              {t('common:recovery_host_addr_label')}
                            </div>
                            <div
                              style={{
                                fontFamily: 'monospace',
                                fontSize: 'var(--text-body-sm)',
                                color: 'var(--text-primary)',
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap',
                              }}
                            >
                              {recoveryInfo.displayAddr}
                            </div>
                          </div>
                          <button
                            type="button"
                            onClick={() => copyToClipboard(recoveryInfo.displayAddr, setCopiedAddr)}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 4,
                              padding: '4px 8px',
                              borderRadius: 4,
                              border: 'none',
                              background: copiedAddr
                                ? 'rgba(39,174,96,0.1)'
                                : 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                              color: copiedAddr ? '#27ae60' : 'var(--accent-primary)',
                              cursor: 'pointer',
                              fontFamily: 'inherit',
                              fontSize: 'var(--text-caption)',
                              flexShrink: 0,
                              transition: 'all 0.15s ease',
                            }}
                          >
                            {copiedAddr ? <Check size={14} /> : <Copy size={14} />}
                            {copiedAddr ? t('common:copied') : t('common:copy')}
                          </button>
                        </div>

                        {/* 可复制 PIN */}
                        <div
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between',
                            padding: '8px 10px',
                            borderRadius: 6,
                            background: 'var(--bg-elevated)',
                            border: '1px solid var(--border-subtle)',
                          }}
                        >
                          <div>
                            <div
                              style={{
                                fontSize: 'var(--text-caption)',
                                color: 'var(--text-tertiary)',
                                marginBottom: 2,
                              }}
                            >
                              {t('common:recovery_host_pin_label')}
                            </div>
                            <div
                              style={{
                                fontFamily: 'monospace',
                                fontSize: 'var(--text-body-sm)',
                                fontWeight: 700,
                                letterSpacing: 4,
                                color: 'var(--accent-primary)',
                              }}
                            >
                              {recoveryInfo.pin}
                            </div>
                          </div>
                          <button
                            type="button"
                            onClick={() => copyToClipboard(recoveryInfo.pin, setCopiedPin)}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 4,
                              padding: '4px 8px',
                              borderRadius: 4,
                              border: 'none',
                              background: copiedPin
                                ? 'rgba(39,174,96,0.1)'
                                : 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                              color: copiedPin ? '#27ae60' : 'var(--accent-primary)',
                              cursor: 'pointer',
                              fontFamily: 'inherit',
                              fontSize: 'var(--text-caption)',
                              flexShrink: 0,
                              transition: 'all 0.15s ease',
                            }}
                          >
                            {copiedPin ? <Check size={14} /> : <Copy size={14} />}
                            {copiedPin ? t('common:copied') : t('common:copy')}
                          </button>
                        </div>

                        <p
                          style={{
                            fontSize: 'var(--text-caption)',
                            color: 'var(--text-tertiary)',
                            margin: '2px 0 0',
                            lineHeight: 1.4,
                          }}
                        >
                          {t('common:recovery_host_manual_note', {
                            defaultValue:
                              'Keep this app open until the transfer completes. The session expires in 5 minutes.',
                          })}
                        </p>
                      </div>
                    )}
                  </div>

                  {/* 过期时间提示 */}
                  <p
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-tertiary)',
                      textAlign: 'center',
                      margin: 0,
                    }}
                  >
                    {t('common:recovery_host_expires')}
                  </p>

                  <Button
                    variant="secondary"
                    onClick={() => switchMode('sync')}
                    style={{ width: '100%' }}
                  >
                    {t('common:recovery_host_cancel')}
                  </Button>
                </motion.div>
              ) : null}
            </>
          ) : (
            <>
              <h2
                style={{
                  fontSize: 'var(--text-card-title)',
                  fontWeight: 700,
                  margin: '0 0 8px',
                  color: 'var(--text-primary)',
                }}
              >
                {t('common:sync_qr_show_title')}
              </h2>
              <p
                style={{
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-secondary)',
                  margin: '0 0 20px',
                  lineHeight: 1.5,
                }}
              >
                {t('common:sync_qr_show_desc')}
              </p>

              {loading ? (
                // 数据未就绪时渲染固定高度的加载占位，与内容区同高 → 卡片高度不再突变
                <div
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 12,
                    minHeight: 360,
                  }}
                >
                  <Loader2 size={32} style={{ animation: 'spin 1s linear infinite' }} />
                  <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                    {t('common:loading')}
                  </span>
                </div>
              ) : error ? (
                <div
                  style={{
                    color: '#e74c3c',
                    fontSize: 'var(--text-body-sm)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    minHeight: 360,
                  }}
                >
                  {error}
                </div>
              ) : info ? (
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ duration: 0.2 }}
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 16,
                    alignItems: 'center',
                    minHeight: 360,
                  }}
                >
                  <div
                    style={{
                      padding: 12,
                      background: '#fff',
                      borderRadius: 12,
                      border: '1px solid var(--border-subtle)',
                    }}
                  >
                    <QRCodeSVG value={info.payload} size={200} level="M" includeMargin />
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
                      <span
                        style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}
                      >
                        {t('common:sync_qr_device_name')}
                      </span>
                      <span
                        style={{
                          fontSize: 'var(--text-body-sm)',
                          fontWeight: 500,
                          color: 'var(--text-primary)',
                        }}
                      >
                        {info.deviceName}
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
                        marginBottom: 8,
                      }}
                    >
                      <span
                        style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}
                      >
                        {t('common:sync_qr_addr')}
                      </span>
                      <span
                        style={{
                          fontFamily: 'monospace',
                          fontSize: 'var(--text-body-sm)',
                          color: 'var(--text-primary)',
                        }}
                      >
                        {info.addr}
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
                      <span
                        style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}
                      >
                        {t('common:sync_qr_fingerprint')}
                      </span>
                      <span
                        style={{
                          fontFamily: 'monospace',
                          fontSize: 'var(--text-body-sm)',
                          color: 'var(--text-primary)',
                          wordBreak: 'break-all',
                          maxWidth: '60%',
                          textAlign: 'right',
                        }}
                      >
                        {info.fingerprint}
                      </span>
                    </div>
                  </div>

                  {info.addr.startsWith('127.') && (
                    <p
                      style={{
                        fontSize: 'var(--text-caption)',
                        color: 'var(--warning)',
                        textAlign: 'center',
                        margin: 0,
                      }}
                    >
                      {t('common:sync_qr_localhost_warning')}
                    </p>
                  )}

                  <Button variant="secondary" onClick={handleClose} style={{ width: '100%' }}>
                    {t('common:close')}
                  </Button>
                </motion.div>
              ) : null}
            </>
          )}
        </Card>
      </motion.div>
    </div>
  );
}
