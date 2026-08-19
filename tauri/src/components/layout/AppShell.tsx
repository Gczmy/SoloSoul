import { useEffect, useLayoutEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { TopFunctionBar } from './TopFunctionBar';
import { MobileBottomNav } from './MobileBottomNav';
import { AppBar } from './AppBar';
import { PairingDialog } from '@/components/sync/PairingDialog';
import { useSettingsStore } from '@/stores/settingsStore';
import { useSyncStore } from '@/stores/syncStore';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';

const FUNCTION_BAR_HEIGHT = 48;

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
}

export function AppShell({ children, title, actions, onBack }: AppShellProps) {
  const isNarrowViewport = useIsNarrowViewport();
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  // 路由导航后内容区滚动位置重置到顶部——滚动发生在 .content（overflow-y: scroll）
  // 而非 window，React Router 不会自动重置，上一页的 scrollTop 会被新页面继承
  // （从长页面中部进入导出/同步等页面时表现为「页面从下方开始」）。
  const contentRef = useRef<HTMLElement>(null);
  const location = useLocation();
  useLayoutEffect(() => {
    if (contentRef.current) contentRef.current.scrollTop = 0;
  }, [location.key]);
  // 窄视口下强制使用底部导航栏
  const effectivePosition = isNarrowViewport ? 'bottom' : sidebarPosition;
  const isTop = effectivePosition === 'top';
  const isHorizontal = isTop || effectivePosition === 'bottom';

  // B 侧入站配对请求：全局挂载监听（响应方用户不在同步页也能弹出配对确认对话框）。
  // 入站 Hello 落库一条新的未信任 peer 记录时，后端 emit sync-pairing-request。
  // 使用 selector 只订阅 incomingPairingRequest，避免整个 store 变化导致全页面重渲染。
  const incomingPairingRequest = useSyncStore((s) => s.incomingPairingRequest);
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    useSyncStore
      .getState()
      .initPairingRequestListener()
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, []);

  // 入站同步完成通知：全局挂载监听（响应方用户不在同步页也能收到「同步完成 + 条数」
  // toast）。与配对请求监听对称，B 侧任意页面都能感知对端完成的同步。
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    useSyncStore
      .getState()
      .initSyncCompletedListener()
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, []);

  const handleIncomingTrust = async () => {
    const s = useSyncStore.getState();
    if (!s.incomingPairingRequest) return;
    // P103: 入站配对确认时绑定握手认证指纹（B 侧配对请求来自握手认证值）
    await s.trustPeer(
      s.incomingPairingRequest.id,
      true,
      s.incomingPairingRequest.fingerprint || undefined,
    );
    await s.loadStatus();
    s.clearIncomingPairingRequest();
  };

  const handleIncomingIgnore = () => {
    useSyncStore.getState().clearIncomingPairingRequest();
  };

  return (
    <div
      className={styles.appShell}
      style={{
        flexDirection: isHorizontal
          ? effectivePosition === 'top'
            ? 'column'
            : 'column-reverse'
          : effectivePosition === 'right'
            ? 'row-reverse'
            : 'row',
      }}
    >
      <AppBar title={title} actions={actions} onBack={onBack} sidebarPosition={effectivePosition} />
      {isNarrowViewport ? (
        <MobileBottomNav />
      ) : isHorizontal ? (
        <TopFunctionBar sidebarPosition={effectivePosition} />
      ) : (
        <SideNavigation />
      )}
      <div
        className={styles.main}
        style={
          isTop
            ? { paddingTop: FUNCTION_BAR_HEIGHT }
            : effectivePosition === 'bottom'
              ? { paddingBottom: FUNCTION_BAR_HEIGHT }
              : undefined
        }
      >
        <main ref={contentRef} className={styles.content}>
          {children}
        </main>
      </div>
      {/* B 侧入站配对请求全局对话框（任意页面可弹出） */}
      <PairingDialog
        isOpen={!!incomingPairingRequest}
        peer={incomingPairingRequest}
        onTrust={handleIncomingTrust}
        onIgnore={handleIncomingIgnore}
      />
    </div>
  );
}
