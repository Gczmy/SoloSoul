import { useCallback, useState } from 'react';
import type { RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { RecoveryDiscoveredHost } from '@/components/recovery/recoveryReceiveTypes';
import { MDNS_DISCOVER_TIMEOUT_MS, PIN_REGEX } from '@/components/recovery/recoveryReceiveTypes';

export interface UseRecoveryManualFormOptions {
  /** 父 hook 的挂载引用：异步回调返回后据此判断组件是否仍挂载。 */
  mountedRef: RefObject<boolean>;
}

/** 手动连接表单与局域网发现（collect 阶段 manual tab）的状态与逻辑。 */
export function useRecoveryManualForm({ mountedRef }: UseRecoveryManualFormOptions) {
  const { t } = useTranslation(['common']);

  // 手动连接表单状态
  const [hostAddr, setHostAddr] = useState('');
  const [pin, setPin] = useState('');
  const [fingerprint, setFingerprint] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);

  // LAN discovery state
  const [scanning, setScanning] = useState(false);
  const [discoveredHosts, setDiscoveredHosts] = useState<RecoveryDiscoveredHost[]>([]);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scanDone, setScanDone] = useState(false);

  // ── 局域网扫描 ──
  const handleScanLan = useCallback(async () => {
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
  }, [scanning, mountedRef, t]);

  const handleSelectHost = useCallback((host: RecoveryDiscoveredHost) => {
    // 安全约束（P001）：mDNS TXT 不再广播 PIN/nonce，仅提供地址与指纹。
    // 选中主机后地址/指纹自动填充，PIN 需由用户在主机屏幕/QR 上查看后手动输入。
    setHostAddr(host.addr);
    setPin('');
    setFingerprint(host.fingerprint);
    setDiscoveredHosts([]);
    setScanDone(false);
    setScanError(null);
  }, []);

  /**
   * 手动输入校验；通过则返回连接信息，不通过则返回错误文案（由调用方展示在全局 error 区）。
   */
  const getPendingInfo = useCallback(
    ():
      | { error: string }
      | { value: { addr: string; pin: string; fingerprint: string; nonce: null } } => {
      if (!hostAddr.trim()) {
        return {
          error: t('common:recovery_receive_addr_required', {
            defaultValue: 'Host address is required',
          }),
        };
      }
      if (!PIN_REGEX.test(pin.trim())) {
        return {
          error: t('common:recovery_receive_invalid_pin', {
            defaultValue: 'PIN must be a 6-digit code',
          }),
        };
      }
      return {
        value: {
          addr: hostAddr.trim(),
          pin: pin.trim(),
          fingerprint: fingerprint.trim(),
          nonce: null, // 手动模式不传 nonce，服务端兼容处理
        },
      };
    },
    [hostAddr, pin, fingerprint, t],
  );

  const reset = useCallback(() => {
    setHostAddr('');
    setPin('');
    setFingerprint('');
    setShowAdvanced(false);
    setDiscoveredHosts([]);
    setScanError(null);
    setScanDone(false);
  }, []);

  return {
    hostAddr,
    pin,
    fingerprint,
    showAdvanced,
    scanning,
    discoveredHosts,
    scanError,
    scanDone,
    setHostAddr,
    setPin,
    setFingerprint,
    setShowAdvanced,
    handleScanLan,
    handleSelectHost,
    getPendingInfo,
    reset,
  };
}
