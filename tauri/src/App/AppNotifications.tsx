import type { ReactNode } from 'react';
import { useAppUpdate } from '@/hooks/useAppUpdate';
import { useOcrFirstInstall } from '@/hooks/useOcrFirstInstall';
import { UpdateBanner } from '@/components/ui/UpdateBanner';
import { OcrInstallBanner } from '@/components/ui/OcrInstallBanner';
import { ShellNotificationsProvider } from '@/components/layout/ShellNotifications';
import { SafSyncIndicator } from '@/components/sync/SafSyncIndicator';
import { PostLoginSetupGuide } from '@/components/guide/PostLoginSetupGuide';
import { ST_SKIPPED_VERSION } from '@/lib/constants';

/** 常驻壳上方的全局通知由同一个所有者装配，路由切换不重建横幅状态。 */
export function AppNotifications({
  isAuthenticated,
  children,
}: {
  isAuthenticated: boolean;
  children: ReactNode;
}) {
  const { updateState, startDownload, cancelDownload, installUpdate, dismissUpdate } =
    useAppUpdate();
  const { showOcrBanner, ocrPhase, progress, error, retryOcrInstall, closeOcrBanner } =
    useOcrFirstInstall();

  const notifications = (
    <>
      {updateState.kind !== 'hidden' && (
        <UpdateBanner
          version={updateState.version}
          state={updateState.kind}
          downloadedBytes={updateState.downloadedBytes}
          totalBytes={updateState.totalBytes}
          progressPercent={updateState.progressPercent}
          transfer={updateState.transfer}
          mandatory={updateState.mandatory}
          error={updateState.error}
          releaseNotes={updateState.releaseNotes}
          checksumWarning={updateState.checksumWarning}
          onUpdate={startDownload}
          onCancel={cancelDownload}
          onInstall={installUpdate}
          onSkip={() => {
            if (!updateState.mandatory) {
              localStorage.setItem(ST_SKIPPED_VERSION, updateState.version);
            }
            dismissUpdate();
          }}
          onClose={dismissUpdate}
        />
      )}
      {showOcrBanner && (
        <OcrInstallBanner
          phase={ocrPhase}
          progress={progress}
          error={error}
          onRetry={retryOcrInstall}
          onClose={closeOcrBanner}
        />
      )}
    </>
  );

  return (
    <ShellNotificationsProvider notifications={notifications}>
      <SafSyncIndicator />
      {isAuthenticated && <PostLoginSetupGuide />}
      {children}
    </ShellNotificationsProvider>
  );
}
