import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Wifi, RefreshCw, Smartphone, Info } from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import { useSyncStore } from '@/stores/syncStore';
import type { SyncPeer } from '@/stores/syncStore';
import { useUiStore } from '@/stores/uiStore';
import type { SyncResult } from '@/lib/ipc';

/**
 * 设备同步页状态机：同步开关/自动同步/发现/信任/忽略/配对等待/忘记确认/QR 对话/冲突对话 + 指南内容。
 * 从 SyncPage 抽出，页面降为纯渲染编排层。
 *
 * P215: 改为字段级选择器（useShallow）订阅——避免整个 store 任意字段变化（如
 * isDiscoveringDevices / lastResult / conflicts）都触发本钩子与 SyncPage 整页重渲染。
 */
export function useSyncPage() {
  const { t } = useTranslation(['settings', 'common']);
  const store = useSyncStore(
    useShallow((s) => ({
      isDiscovering: s.isDiscovering,
      syncEnabled: s.syncEnabled,
      autoSyncEnabled: s.autoSyncEnabled,
      localFingerprint: s.localFingerprint,
      connectedPeers: s.connectedPeers,
      isLoading: s.isLoading,
      error: s.error,
      lastResult: s.lastResult,
      recentResults: s.recentResults,
      discoveredDevices: s.discoveredDevices,
      isDiscoveringDevices: s.isDiscoveringDevices,
      listenAddr: s.listenAddr,
      conflicts: s.conflicts,
      selectedConflict: s.selectedConflict,
      pairingPendingPeerId: s.pairingPendingPeerId,
      pairingPendingSasCode: s.pairingPendingSasCode,
      discoverDevices: s.discoverDevices,
      syncWithDevice: s.syncWithDevice,
      trustPeer: s.trustPeer,
      forgetPeer: s.forgetPeer,
      enable: s.enable,
      setAutoSyncEnabled: s.setAutoSyncEnabled,
      loadStatus: s.loadStatus,
      loadListenAddr: s.loadListenAddr,
      loadAutoSyncStatus: s.loadAutoSyncStatus,
      loadConflicts: s.loadConflicts,
      resolveConflict: s.resolveConflict,
      clearPairingPending: s.clearPairingPending,
      initNsdFailedListener: s.initNsdFailedListener,
    })),
  );
  const [manualAddr, setManualAddr] = useState('');
  const [ignoredPeerIds, setIgnoredPeerIds] = useState<Set<string>>(new Set());
  const [activityOpen, setActivityOpen] = useState(false);
  const [conflictDialogOpen, setConflictDialogOpen] = useState(false);
  const [showQrDialogOpen, setShowQrDialogOpen] = useState(false);
  const [scanQrDialogOpen, setScanQrDialogOpen] = useState(false);
  /** A 侧配对等待态：idle=未进入等待 / waiting=自动重试中 / failed=对方尚未确认 */
  const [pairWaitState, setPairWaitState] = useState<'idle' | 'waiting' | 'failed'>('idle');
  /** 「去配对」按钮手动指定的配对目标（已知设备未信任行）。 */
  const [pairTarget, setPairTarget] = useState<SyncPeer | null>(null);
  /** 「忘记」二次确认的目标设备。 */
  const [forgetTarget, setForgetTarget] = useState<SyncPeer | null>(null);
  /** 配对自动重试循环的取消标记。 */
  const retryCancelledRef = useRef(false);

  const syncGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_sync_title', { defaultValue: 'Device Sync Guide' }),
        steps: [
          {
            icon: Wifi,
            title: t('common:guide_sync_step1_title', { defaultValue: 'Enable Sync' }),
            description:
              t('common:guide_sync_step1_desc', { defaultValue: 'Turn on sync to make your device discoverable and start listening on a local port. Both devices must be on the same Wi-Fi network.' }),
          },
          {
            icon: RefreshCw,
            title: t('common:guide_sync_step2_title', { defaultValue: 'Discover & Pair' }),
            description:
              t('common:guide_sync_step2_desc', { defaultValue: 'Tap Discover to scan for nearby devices. Tap Sync on a discovered device to pair, then verify the fingerprint to trust it.' }),
          },
          {
            icon: Smartphone,
            title: t('common:guide_sync_step3_title', { defaultValue: 'Automatic Sync' }),
            description:
              t('common:guide_sync_step3_desc', { defaultValue: 'Enable Automatic Sync to keep data in sync when the app is in the foreground, on data changes, and periodically.' }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_device_sync', { defaultValue: 'Device Sync' }),
            description:
              t('common:guide_help_device_sync_desc', { defaultValue: 'Pair devices over LAN and keep data in sync' }),
            href: '/help?id=device-sync',
          },
        ],
      },
    ],
    [t],
  );

  // B 侧入站配对请求：AppShell 全局对话框已处理，同步页的 pendingPeer 自动弹窗需让位，
  // 避免同一 peer 同时弹出 SyncPage 与 AppShell 两个配对对话框（双弹窗叠加）。
  const hasIncomingRequest = useSyncStore((s) => s.incomingPairingRequest !== null);

  /** 自动检测的未信任 peer（首次同步失败后对端已被 record_peer 持久化）。
   *  有入站配对请求（B 侧）时不自动弹窗，由 AppShell 全局对话框统一处理，避免双弹窗。 */
  const pendingPeer = useMemo(() => {
    if (hasIncomingRequest) return null;
    return (
      store.connectedPeers.find((p) => !p.trusted && !ignoredPeerIds.has(p.id) && p.fingerprint) ||
      null
    );
  }, [store.connectedPeers, ignoredPeerIds, hasIncomingRequest]);

  /** A 侧配对目标：pairingPendingPeerId 对应的 peer（等待对端确认）。
   *  若 store 携带本次握手派生的 SAS 验证码则附加到 peer 上，配对卡片据此展示。 */
  const pairingPendingPeer = useMemo(() => {
    if (!store.pairingPendingPeerId) return null;
    const peer = store.connectedPeers.find((p) => p.id === store.pairingPendingPeerId) || null;
    if (!peer) return null;
    if (store.pairingPendingSasCode) {
      return { ...peer, sasCode: store.pairingPendingSasCode };
    }
    return peer;
  }, [store.connectedPeers, store.pairingPendingPeerId, store.pairingPendingSasCode]);

  // ⚡ 使用 getState() 避免闭包捕获整个 store 导致无限重触发
  const loadStatus = useCallback(async () => {
    const s = useSyncStore.getState();
    await Promise.all([
      s.loadStatus(),
      s.loadListenAddr(),
      s.loadAutoSyncStatus(),
      s.loadConflicts(),
    ]);
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  // P0#4: 页面停留期间每 15s 轮询同步状态/监听地址/冲突——设备上线、离线、mDNS
  // 发现链中断不会自行触发事件，仅靠挂载/操作/事件刷新会让列表停留过期快照
  // （两台设备都在线却一直显示离线）。轮询使用 getState() 避免闭包捕获旧 store。
  useEffect(() => {
    const timer = setInterval(() => {
      const s = useSyncStore.getState();
      void s.loadStatus();
      void s.loadListenAddr();
      void s.loadConflicts();
    }, 15_000);
    return () => clearInterval(timer);
  }, []);

  // 监听移动端 NSD 注册失败事件：后端已回滚为禁用，重读状态避免开关 UI 漂移
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    useSyncStore
      .getState()
      .initNsdFailedListener()
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, []);

  // 组件卸载时取消在途的配对自动重试循环
  useEffect(() => {
    retryCancelledRef.current = false;
    return () => {
      retryCancelledRef.current = true;
    };
  }, []);

  // 使用 selector 只监听 syncEnabled 变化，避免 store 全局变化时误触发
  const syncEnabled = useSyncStore((s) => s.syncEnabled);
  useEffect(() => {
    if (syncEnabled) {
      useSyncStore.getState().discoverDevices(5000);
    }
  }, [syncEnabled]);

  // 使用 getState() 读取最新状态 + isLoading 防抖：
  // 1) 避免渲染闭包捕获的旧 syncEnabled 导致连点时目标值反转（点"禁用"实际执行"启用"）；
  // 2) 上一个切换在途时忽略新点击，防止并发 enable 交错把 isLoading 卡在 true。
  const handleToggleSync = async () => {
    const s = useSyncStore.getState();
    if (s.isLoading) return;
    await s.enable(!s.syncEnabled);
  };

  const handleToggleAutoSync = async () => {
    const s = useSyncStore.getState();
    if (s.isLoading) return;
    await s.setAutoSyncEnabled(!s.autoSyncEnabled);
  };

  const handleDiscover = async () => {
    await store.discoverDevices(5000);
  };

  const handleSyncWithDevice = async (deviceId: string) => {
    await store.syncWithDevice(deviceId);
  };

  /** 普通信任（已知设备行「去配对」/ 自动检测 pendingPeer）。 */
  const handleTrustPending = async () => {
    const peer = pairTarget || pendingPeer;
    if (!peer) return;
    // P103: 配对确认时绑定握手认证指纹（peer.fingerprint 来自握手认证值）
    await store.trustPeer(peer.id, true, peer.fingerprint || undefined);
    setPairTarget(null);
  };

  const handleIgnorePending = () => {
    const peer = pairTarget || pendingPeer;
    if (!peer) return;
    setIgnoredPeerIds((prev) => new Set(prev).add(peer.id));
    setPairTarget(null);
  };

  /**
   * A 侧配对确认：信任对端（打通反向）→ 进入等待态并自动重试同步（每 3s 一次，最多 5 次）。
   * 成功则清除配对状态并关闭；超时提示「对方尚未确认」。
   * 注意：仅用于 A 侧等待流程（pairingPendingPeerId 存在）；去配对/自动检测走 handleTrustPending。
   */
  const handleConfirmPairing = useCallback(async () => {
    const s = useSyncStore.getState();
    if (!s.pairingPendingPeerId) {
      return;
    }
    // P103: A 侧配对确认时绑定握手认证指纹（pairingPendingPeer 来自连接记录/握手认证值）
    await s.trustPeer(s.pairingPendingPeerId, true, pairingPendingPeer?.fingerprint || undefined);
    setPairWaitState('waiting');

    const addr = s.pairingPendingAddr || pairingPendingPeer?.addr || '';
    if (!addr) {
      setPairWaitState('failed');
      return;
    }

    retryCancelledRef.current = false;
    for (let attempt = 0; attempt < 5; attempt++) {
      await new Promise((r) => setTimeout(r, 3000));
      if (retryCancelledRef.current) return;
      const st = useSyncStore.getState();
      // 配对状态已被清除（成功/取消）→ 退出循环
      if (!st.pairingPendingPeerId) {
        setPairWaitState('idle');
        return;
      }
      await st.syncWithDevice(addr);
      const after = useSyncStore.getState();
      // 成功：lastResult 有值 → 清除配对状态并关闭等待。
      // 注意：syncWithDevice 成功路径不清 pairingPending，需在此显式清除，
      // 否则下一轮检测 pairingPendingPeerId 仍非空、5 次后误报「对方尚未确认」。
      if (after.lastResult) {
        after.clearPairingPending();
        setPairWaitState('idle');
        return;
      }
      // 非配对错误（连接失败、对端离线等）：syncWithDevice 已把 raw 写入 store.error，
      // 由页面错误横幅（resolveBackendErrorMessage）展示真实原因；这里关闭等待对话框并
      // 清除配对状态，避免误显示「对方尚未确认」。pairing_pending 分支 error 为 null，不会走到。
      if (after.error) {
        after.clearPairingPending();
        setPairWaitState('idle');
        return;
      }
      if (retryCancelledRef.current) return;
    }
    // 5 次后仍未成功 → 对方尚未确认
    if (useSyncStore.getState().pairingPendingPeerId) {
      setPairWaitState('failed');
    } else {
      setPairWaitState('idle');
    }
  }, [pairingPendingPeer]);

  /** 取消 A 侧配对等待（停止自动重试并关闭）。 */
  const handleCancelPairing = useCallback(() => {
    retryCancelledRef.current = true;
    useSyncStore.getState().clearPairingPending();
    setPairWaitState('idle');
  }, []);

  /** 「去配对」按钮：指定某个未信任已知设备为配对目标。 */
  const handleOpenPairTarget = (peer: SyncPeer) => {
    setPairTarget(peer);
  };

  /** 「忘记」二次确认。 */
  const handleForgetRequest = (peer: SyncPeer) => {
    setForgetTarget(peer);
  };

  const handleForgetConfirm = async () => {
    if (!forgetTarget) return;
    await store.forgetPeer(forgetTarget.id);
    setForgetTarget(null);
  };

  const handleForgetCancel = () => {
    setForgetTarget(null);
  };

  const handleOpenConflictDialog = () => {
    setConflictDialogOpen(true);
  };

  const handleScanSync = async (addr: string) => {
    // 扫码后台执行：syncWithDevice 内部处理 pairing_pending（进入双向确认配对流程），
    // 成功则写入 lastResult，由下方 lastResult 监听统一弹「同步完成」toast。
    await store.syncWithDevice(addr);
  };

  // 后台同步完成提示：syncWithDevice 成功（扫码后台执行 / 手动同步 / 配对重试成功）
  // 写入 lastResult 时弹 toast——替代扫码对话框内阻塞式「同步完成」界面，
  // 用户无需等待即可继续操作。ref 初始化为当前值避免页面挂载时误弹。
  // 入站结果（inbound: true，来自 sync-completed 事件）已由 syncStore 全局弹带条数
  // toast，这里跳过避免同一会话双弹。
  const prevLastResultRef = useRef<SyncResult | null>(store.lastResult);
  useEffect(() => {
    const cur = store.lastResult;
    const prev = prevLastResultRef.current;
    prevLastResultRef.current = cur;
    if (cur && cur !== prev && !cur.inbound) {
      useUiStore.getState().showToast({
        type: 'success',
        message: t('common:sync_qr_success_sync', { defaultValue: 'Sync completed' }),
      });
    }
  }, [store.lastResult, t]);

  return {
    store,
    manualAddr,
    setManualAddr,
    pendingPeer,
    pairingPendingPeer,
    pairWaitState,
    pairTarget,
    forgetTarget,
    activityOpen,
    setActivityOpen,
    conflictDialogOpen,
    setConflictDialogOpen,
    showQrDialogOpen,
    setShowQrDialogOpen,
    scanQrDialogOpen,
    setScanQrDialogOpen,
    syncGuidePages,
    loadStatus,
    handleToggleSync,
    handleToggleAutoSync,
    handleDiscover,
    handleSyncWithDevice,
    handleTrustPending,
    handleIgnorePending,
    handleConfirmPairing,
    handleCancelPairing,
    handleOpenPairTarget,
    handleForgetRequest,
    handleForgetConfirm,
    handleForgetCancel,
    handleOpenConflictDialog,
    handleScanSync,
  };
}
