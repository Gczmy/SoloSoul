import { useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { ToggleSwitch } from '@/components/ui/ToggleSwitch';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { Wifi, WifiOff } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { useSyncPage } from './useSyncPage';
import { ICON_SIZE } from '@/lib/constants';
import type { DiscoveredDevice } from '@/stores/syncStore';
import { ConflictPanel } from './ConflictPanel';
import { PairingPanel } from './PairingPanel';
import { DeviceListPanel } from './DeviceListPanel';
import { SyncHistoryPanel } from './SyncHistoryPanel';

/**
 * 设备同步页：渲染编排层。
 * 状态机/事件处理器收敛于 useSyncPage hook；四面板（Conflict/Pairing/DeviceList/SyncHistory）
 * 为纯展示组件，数据与回调经本页从 useSyncPage 透传。
 */
export function SyncPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const backTo = (location.state as { from?: string } | null)?.from;
  const { t } = useTranslation(['settings', 'common']);
  // 已知设备详情弹窗目标 id（列表卡片点击后打开）。
  // peer 对象从 store.connectedPeers 派生而非存快照：点击「撤销信任/信任并配对」后
  // trustPeer → loadStatus 刷新 connectedPeers，弹窗内容立即反映最新信任状态，
  // 无需重新进入卡片才能看到变化。
  const [detailPeerId, setDetailPeerId] = useState<string | null>(null);
  // 已发现设备详情弹窗目标（点击已发现设备卡片后打开）。
  // 若该设备匹配到已知设备（按指纹），则复用已知设备详情（含信任/忘记操作）；
  // 否则以「新设备」形态展示并仅提供「立即同步」。
  const [discoveredDetail, setDiscoveredDetail] = useState<DiscoveredDevice | null>(null);
  const {
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
    handleToggleUiPrefsSync,
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
  } = useSyncPage();

  // 详情弹窗 peer 由 store.connectedPeers 派生（见上方 detailPeerId 注释）。
  // 已发现设备先按指纹匹配已知设备（同一设备指纹恒等）——匹配到则复用已知设备
  // 详情（含信任状态与操作按钮）；未匹配（新设备）则走 discovered 形态。
  const matchedKnownPeer = discoveredDetail
    ? (store.connectedPeers.find(
        (p) => p.fingerprint && p.fingerprint === discoveredDetail.fingerprint,
      ) ?? null)
    : null;
  const detailPeer =
    (detailPeerId ? store.connectedPeers.find((p) => p.id === detailPeerId) : null) ||
    matchedKnownPeer;
  const detailDiscovered = detailPeer ? null : discoveredDetail;

  const handleCloseDetail = () => {
    setDetailPeerId(null);
    setDiscoveredDetail(null);
  };

  const handleOpenDiscoveredDetail = (device: DiscoveredDevice) => {
    // 关闭已知设备详情，仅展示本次点击的设备
    setDetailPeerId(null);
    setDiscoveredDetail(device);
  };

  // 配对对话框目标：A 侧等待流程优先（pairingPendingPeer），否则「去配对」手动目标，否则自动检测
  const activePairPeer = pairingPendingPeer || pairTarget || pendingPeer;
  const isWaitingFlow = !!pairingPendingPeer;
  const confirmLabel = isWaitingFlow
    ? t('settings:sync_pairing_confirm_wait', { defaultValue: 'Confirm & Wait' })
    : undefined;

  return (
    <PageShell
      title={t('settings:sync', { defaultValue: 'Device Sync' })}
      onBack={() => navigate(backTo || '/home', { replace: true })}
      actions={<PageGuideButton pages={syncGuidePages} />}
    >
      <PageContainer variant="xs" gap="default">
        <ConflictPanel
          conflicts={store.conflicts}
          selectedConflict={store.selectedConflict}
          dialogOpen={conflictDialogOpen}
          isLoading={store.isLoading}
          onOpenDialog={handleOpenConflictDialog}
          onCloseDialog={() => setConflictDialogOpen(false)}
          onResolve={(id, strategy) => store.resolveConflict(id, strategy)}
        />

        {/* Status card（P041: 提取为子组件） */}
        <SyncStatusCard
          store={store}
          t={t}
          onToggleSync={handleToggleSync}
          onToggleAutoSync={handleToggleAutoSync}
          onToggleUiPrefsSync={handleToggleUiPrefsSync}
        />

        <PairingPanel
          syncEnabled={store.syncEnabled}
          isLoading={store.isLoading}
          pairPeer={activePairPeer}
          isWaitingFlow={isWaitingFlow}
          pairWaitState={pairWaitState}
          confirmLabel={confirmLabel}
          showQrOpen={showQrDialogOpen}
          scanQrOpen={scanQrDialogOpen}
          onShowQrOpen={setShowQrDialogOpen}
          onScanQrOpen={setScanQrDialogOpen}
          onScanSync={handleScanSync}
          onTrust={isWaitingFlow ? handleConfirmPairing : handleTrustPending}
          onIgnore={isWaitingFlow ? handleCancelPairing : handleIgnorePending}
          onCancelWaiting={handleCancelPairing}
        />

        <DeviceListPanel
          syncEnabled={store.syncEnabled}
          isLoading={store.isLoading}
          isDiscoveringDevices={store.isDiscoveringDevices}
          discoveredDevices={store.discoveredDevices}
          connectedPeers={store.connectedPeers}
          manualAddr={manualAddr}
          lastResult={store.lastResult}
          error={store.error}
          forgetTarget={forgetTarget}
          detailPeer={detailPeer}
          detailDiscovered={detailDiscovered}
          onManualAddrChange={setManualAddr}
          onDiscover={handleDiscover}
          onSyncWithDevice={handleSyncWithDevice}
          onTrustPeer={(id) => store.trustPeer(id, false)}
          onOpenPairTarget={handleOpenPairTarget}
          onForgetRequest={handleForgetRequest}
          onForgetConfirm={handleForgetConfirm}
          onForgetCancel={handleForgetCancel}
          onRefresh={loadStatus}
          onOpenDiscoveredDetail={handleOpenDiscoveredDetail}
          onOpenDetail={(peer) => {
            // 与 handleOpenDiscoveredDetail 对称：已知设备卡片点击时清空发现详情，
            // 避免两个详情状态并存（模态下不会发生，防御性保持状态互斥）
            setDiscoveredDetail(null);
            setDetailPeerId(peer.id);
          }}
          onCloseDetail={handleCloseDetail}
          onSyncDiscovered={handleSyncWithDevice}
        />

        <SyncHistoryPanel
          activityOpen={activityOpen}
          recentResults={store.recentResults}
          onToggleActivity={() => setActivityOpen((v) => !v)}
        />
      </PageContainer>
    </PageShell>
  );
}

/** P041: 同步状态卡片——从主组件提取，缩短主组件体积。 */
function SyncStatusCard({
  store,
  t,
  onToggleSync,
  onToggleAutoSync,
  onToggleUiPrefsSync,
}: {
  store: ReturnType<typeof useSyncPage>['store'];
  t: ReturnType<typeof useTranslation>['t'];
  onToggleSync: () => void;
  onToggleAutoSync: () => void;
  onToggleUiPrefsSync: () => void;
}) {
  return (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div
            style={{
              width: 40,
              height: 40,
              borderRadius: 10,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: store.syncEnabled ? 'rgba(39,174,96,0.12)' : 'rgba(128,128,128,0.1)',
            }}
          >
            {store.syncEnabled ? (
              <Wifi size={ICON_SIZE.xl} color="#27ae60" />
            ) : (
              <WifiOff size={ICON_SIZE.xl} color="#888" />
            )}
          </div>
          <div>
            <div style={{ fontSize: 'var(--text-card-title)', fontWeight: 600 }}>
              {store.syncEnabled
                ? t('settings:sync_enabled', { defaultValue: 'Sync Enabled' })
                : t('settings:sync_disabled', { defaultValue: 'Sync Disabled' })}
            </div>
            <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {t('settings:sync_known_devices', {
                count: store.connectedPeers.length,
                defaultValue: `${store.connectedPeers.length} device(s) known`,
              })}
            </div>
          </div>
        </div>
        <button
          onClick={onToggleSync}
          disabled={store.isLoading}
          className={store.syncEnabled ? 'interactive-toolbar-selected' : 'interactive-toolbar'}
          style={{
            padding: '8px 16px',
            borderRadius: 8,
            borderWidth: 1,
            borderStyle: 'solid',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            cursor: store.isLoading ? 'default' : 'pointer',
            opacity: store.isLoading ? 0.6 : 1,
            fontFamily: 'inherit',
          }}
        >
          {store.syncEnabled
            ? t('settings:sync_disable', { defaultValue: 'Disable' })
            : t('settings:sync_enable', { defaultValue: 'Enable' })}
        </button>
      </div>

      {/* Auto-sync toggle */}
      <div
        style={{
          marginTop: 12,
          padding: 10,
          borderRadius: 8,
          background: 'var(--bg-toolbar)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div>
          <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
            {t('settings:sync_auto', { defaultValue: 'Automatic Sync' })}
          </div>
          <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {t('settings:sync_auto_desc', {
              defaultValue: 'Sync automatically on foreground, data changes, and periodically.',
            })}
          </div>
        </div>
        <ToggleSwitch
          checked={store.autoSyncEnabled}
          onChange={onToggleAutoSync}
          disabled={!store.syncEnabled || store.isLoading}
        />
      </div>

      {/* 账户设置偏好（主题/主题色等 UI 外观）是否随设备同步 */}
      <div
        style={{
          marginTop: 8,
          padding: 10,
          borderRadius: 8,
          background: 'var(--bg-toolbar)',
          display: 'flex',
          alignItems: 'flex-start',
          gap: 10,
          cursor: !store.syncEnabled || store.isLoading ? 'default' : 'pointer',
          opacity: !store.syncEnabled ? 0.55 : 1,
        }}
        onClick={() => {
          if (store.syncEnabled && !store.isLoading) onToggleUiPrefsSync();
        }}
      >
        <div style={{ paddingTop: 1 }}>
          <SelectCheckbox
            checked={store.uiPrefsSyncEnabled}
            size={16}
            borderRadius={4}
            disabled={!store.syncEnabled || store.isLoading}
          />
        </div>
        <div>
          <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
            {t('settings:sync_ui_prefs', { defaultValue: 'Sync UI preferences' })}
          </div>
          <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {t('settings:sync_ui_prefs_desc', {
              defaultValue:
                'Sync appearance settings (theme, accent color, etc.) with other devices.',
            })}
          </div>
        </div>
      </div>

      {(store.localFingerprint || (store.syncEnabled && store.listenAddr)) && (
        <div
          style={{
            marginTop: 8,
            padding: 8,
            borderRadius: 8,
            background: 'var(--bg-toolbar)',
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
            wordBreak: 'break-all',
          }}
        >
          {/* 三个本地设备信息行（设备名/指纹/监听地址）合并为单容器紧凑排列，
              行间距 6px，不再作为独立卡片造成视觉分离 */}
          {store.localFingerprint && (
            <div style={{ lineHeight: 1.6 }}>
              <strong>
                {t('settings:sync_your_device_name', { defaultValue: 'Your device name' })}:
              </strong>{' '}
              {`SoloSoul-${store.localFingerprint.slice(0, 8)}`}
            </div>
          )}
          {store.localFingerprint && (
            <div style={{ marginTop: 6, lineHeight: 1.6 }}>
              <strong>
                {t('settings:sync_your_fingerprint', { defaultValue: 'Your fingerprint' })}:
              </strong>{' '}
              {store.localFingerprint}
            </div>
          )}
          {store.syncEnabled && store.listenAddr && (
            <div style={{ marginTop: 6, lineHeight: 1.6 }}>
              <strong>{t('settings:sync_your_addr', { defaultValue: 'Your listen address' })}:</strong>{' '}
              {store.listenAddr}
            </div>
          )}
        </div>
      )}
    </Card>
  );
}
