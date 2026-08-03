import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { ToggleSwitch } from '@/components/ui/ToggleSwitch';
import { Wifi, WifiOff } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { useSyncPage } from './useSyncPage';
import { ICON_SIZE } from '@/lib/constants';
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

  // 配对对话框目标：A 侧等待流程优先（pairingPendingPeer），否则「去配对」手动目标，否则自动检测
  const activePairPeer = pairingPendingPeer || pairTarget || pendingPeer;
  const isWaitingFlow = !!pairingPendingPeer;
  const confirmLabel = isWaitingFlow
    ? t('settings:sync_pairing_confirm_wait', { defaultValue: 'Confirm & Wait' })
    : undefined;

  return (
    <AppShell
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
          onManualAddrChange={setManualAddr}
          onDiscover={handleDiscover}
          onSyncWithDevice={handleSyncWithDevice}
          onTrustPeer={(id) => store.trustPeer(id, false)}
          onOpenPairTarget={handleOpenPairTarget}
          onForgetRequest={handleForgetRequest}
          onForgetConfirm={handleForgetConfirm}
          onForgetCancel={handleForgetCancel}
          onRefresh={loadStatus}
        />

        <SyncHistoryPanel
          activityOpen={activityOpen}
          recentResults={store.recentResults}
          onToggleActivity={() => setActivityOpen((v) => !v)}
        />
      </PageContainer>
    </AppShell>
  );
}

/** P041: 同步状态卡片——从主组件提取，缩短主组件体积。 */
function SyncStatusCard({
  store,
  t,
  onToggleSync,
  onToggleAutoSync,
}: {
  store: ReturnType<typeof useSyncPage>['store'];
  t: ReturnType<typeof useTranslation>['t'];
  onToggleSync: () => void;
  onToggleAutoSync: () => void;
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

      {store.localFingerprint && (
        <div
          style={{
            marginTop: 12,
            padding: 10,
            borderRadius: 8,
            background: 'var(--bg-toolbar)',
            fontSize: 'var(--text-caption)',
            fontFamily: 'monospace',
            wordBreak: 'break-all',
          }}
        >
          <strong>
            {t('settings:sync_your_fingerprint', { defaultValue: 'Your fingerprint' })}:
          </strong>{' '}
          {store.localFingerprint}
        </div>
      )}
      {store.syncEnabled && store.listenAddr && (
        <div
          style={{
            marginTop: 8,
            padding: 10,
            borderRadius: 8,
            background: 'var(--bg-toolbar)',
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
            fontFamily: 'monospace',
            wordBreak: 'break-all',
          }}
        >
          <strong>{t('settings:sync_your_addr', { defaultValue: 'Your listen address' })}:</strong>{' '}
          {store.listenAddr}
        </div>
      )}
    </Card>
  );
}
