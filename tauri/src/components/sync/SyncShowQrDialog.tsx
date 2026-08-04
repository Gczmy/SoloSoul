import { useEffect, useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { motion } from 'framer-motion';
import { X } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { translateRustError } from '@/lib/rustErrors';
import { SyncQrTabSwitcher, type QrMode } from '@/components/sync/SyncQrTabSwitcher';
import { SyncQrContent, type SyncQrInfo } from '@/components/sync/SyncQrContent';
import { RecoveryQrContent, type RecoveryHostInfo } from '@/components/sync/RecoveryQrContent';

interface SyncShowQrDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

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
          <SyncQrTabSwitcher t={t} isRecovery={isRecovery} onSelect={switchMode} />

          {isRecovery ? (
            <RecoveryQrContent
              t={t}
              loading={recoveryLoading}
              error={recoveryError}
              info={recoveryInfo}
              manualOpen={manualOpen}
              copiedAddr={copiedAddr}
              copiedPin={copiedPin}
              onToggleManual={() => setManualOpen(!manualOpen)}
              onCopyAddr={() => {
                if (recoveryInfo) copyToClipboard(recoveryInfo.displayAddr, setCopiedAddr);
              }}
              onCopyPin={() => {
                if (recoveryInfo) copyToClipboard(recoveryInfo.pin, setCopiedPin);
              }}
              onCancel={() => switchMode('sync')}
            />
          ) : (
            <SyncQrContent
              t={t}
              loading={loading}
              error={error}
              info={info}
              onClose={handleClose}
            />
          )}
        </Card>
      </motion.div>
    </div>
  );
}
