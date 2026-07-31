import { useState, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Wifi, RefreshCw, Smartphone, Info } from 'lucide-react';
import { useSyncStore } from '@/stores/syncStore';

/**
 * 设备同步页状态机：同步开关/自动同步/发现/信任/忽略/QR 对话/冲突对话 + 指南内容。
 * 从 SyncPage 抽出，页面降为纯渲染编排层。
 */
export function useSyncPage() {
  const { t } = useTranslation(['settings', 'common']);
  const store = useSyncStore();
  const [manualAddr, setManualAddr] = useState('');
  const [ignoredPeerIds, setIgnoredPeerIds] = useState<Set<string>>(new Set());
  const [activityOpen, setActivityOpen] = useState(false);
  const [conflictDialogOpen, setConflictDialogOpen] = useState(false);
  const [showQrDialogOpen, setShowQrDialogOpen] = useState(false);
  const [scanQrDialogOpen, setScanQrDialogOpen] = useState(false);

  const syncGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_sync_title') ?? 'Device Sync Guide',
        steps: [
          {
            icon: Wifi,
            title: t('common:guide_sync_step1_title') ?? 'Enable Sync',
            description:
              t('common:guide_sync_step1_desc') ??
              'Turn on sync to make your device discoverable and start listening on a local port. Both devices must be on the same Wi-Fi network.',
          },
          {
            icon: RefreshCw,
            title: t('common:guide_sync_step2_title') ?? 'Discover & Pair',
            description:
              t('common:guide_sync_step2_desc') ??
              'Tap Discover to scan for nearby devices. Tap Sync on a discovered device to pair, then verify the fingerprint to trust it.',
          },
          {
            icon: Smartphone,
            title: t('common:guide_sync_step3_title') ?? 'Automatic Sync',
            description:
              t('common:guide_sync_step3_desc') ??
              'Enable Automatic Sync to keep data in sync when the app is in the foreground, on data changes, and periodically.',
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_device_sync') ?? 'Device Sync',
            description:
              t('common:guide_help_device_sync_desc') ??
              'Pair devices over LAN and keep data in sync',
            href: '/help?id=device-sync',
          },
        ],
      },
    ],
    [t],
  );

  const pendingPeer = useMemo(() => {
    return (
      store.connectedPeers.find((p) => !p.trusted && !ignoredPeerIds.has(p.id) && p.fingerprint) ||
      null
    );
  }, [store.connectedPeers, ignoredPeerIds]);

  // ⚡ 使用 getState() 避免闭包捕获整个 store 导致无限重触发
  const loadStatus = useCallback(async () => {
    const s = useSyncStore.getState();
    await Promise.all([
      s.loadStatus(),
      s.loadListenPort(),
      s.loadAutoSyncStatus(),
      s.loadConflicts(),
    ]);
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

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

  const handleTrustPending = async () => {
    if (!pendingPeer) return;
    await store.trustPeer(pendingPeer.id, true);
  };

  const handleIgnorePending = () => {
    if (!pendingPeer) return;
    setIgnoredPeerIds((prev) => new Set(prev).add(pendingPeer.id));
  };

  const handleOpenConflictDialog = () => {
    setConflictDialogOpen(true);
  };

  const handleScanSync = async (addr: string) => {
    await store.syncWithDevice(addr);
  };

  return {
    store,
    manualAddr,
    setManualAddr,
    pendingPeer,
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
    handleOpenConflictDialog,
    handleScanSync,
  };
}
