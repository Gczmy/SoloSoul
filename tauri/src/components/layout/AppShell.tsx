import { useEffect, useLayoutEffect, useRef, type CSSProperties } from 'react';
import { trackAsyncListener } from '@/lib/asyncListener';
import { useLocation } from 'react-router-dom';
import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { TopFunctionBar } from './TopFunctionBar';
import { MobileBottomNav } from './MobileBottomNav';
import { AppBar } from './AppBar';
import { PairingDialog } from '@/components/sync/PairingDialog';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import { useSyncStore } from '@/stores/syncStore';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';

const FUNCTION_BAR_HEIGHT = 48;

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  primaryActions?: React.ReactNode;
  onBack?: () => void;
}

export function AppShell({ children, title, actions, primaryActions, onBack }: AppShellProps) {
  const isNarrowViewport = useIsNarrowViewport();
  const titlebarHeight = useNativeWindowStore((s) => s.titlebarHeight);
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const isWindows = useNativeWindowStore((s) => s.isWindows);
  const trafficLightsRight = useNativeWindowStore((s) => s.trafficLightsRight);
  const appbarHeight = isNarrowViewport
    ? 48
    : isMacOS
      ? titlebarHeight || 52
      : isWindows
        ? titlebarHeight || 40
        : 48;
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const sidebarExpanded = useUiStore((s) => s.sidebarExpanded);
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
  // 折叠侧栏仍完整容纳交通灯，并保留右侧留白；全屏隐藏按钮时也不缩回窄条。
  const collapsedWidth =
    isMacOS && effectivePosition === 'left' ? Math.max(96, Math.ceil(trafficLightsRight + 16)) : 64;
  const sidebarWidth = sidebarExpanded ? Math.max(232, collapsedWidth) : collapsedWidth;

  // Portal 快捷卡片与固定聊天面板也读取同一尺寸，避免展开后仍使用 48px 偏移。
  useLayoutEffect(() => {
    const root = document.documentElement;
    const values: Record<string, string> = {
      '--sidebar-width': `${sidebarWidth}px`,
      '--shell-content-left': `${effectivePosition === 'left' ? sidebarWidth : 0}px`,
      '--shell-content-right': `${effectivePosition === 'right' ? sidebarWidth : 0}px`,
      '--shell-content-top': `${appbarHeight + (isTop ? FUNCTION_BAR_HEIGHT : 0)}px`,
      '--shell-content-bottom': `${isNarrowViewport ? 56 : effectivePosition === 'bottom' ? 48 : 0}px`,
      '--shell-page-padding': isNarrowViewport ? '16px' : '24px',
      '--shell-content-height':
        'calc(100dvh - var(--shell-content-top) - var(--shell-content-bottom))',
    };
    Object.entries(values).forEach(([key, value]) => root.style.setProperty(key, value));
    return () => Object.keys(values).forEach((key) => root.style.removeProperty(key));
  }, [sidebarWidth, effectivePosition, isHorizontal, isTop, isNarrowViewport, appbarHeight]);

  // B 侧入站配对请求：全局挂载监听（响应方用户不在同步页也能弹出配对确认对话框）。
  // 入站 Hello 落库一条新的未信任 peer 记录时，后端 emit sync-pairing-request。
  // 使用 selector 只订阅 incomingPairingRequest，避免整个 store 变化导致全页面重渲染。
  const incomingPairingRequest = useSyncStore((s) => s.incomingPairingRequest);
  useEffect(() => trackAsyncListener(useSyncStore.getState().initPairingRequestListener()), []);

  // 入站同步完成通知：全局挂载监听（响应方用户不在同步页也能收到「同步完成 + 条数」
  // toast）。与配对请求监听对称，B 侧任意页面都能感知对端完成的同步。
  useEffect(() => trackAsyncListener(useSyncStore.getState().initSyncCompletedListener()), []);

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
      data-navigation={effectivePosition}
      style={
        {
          '--appbar-height': `${appbarHeight}px`,
          flexDirection: isHorizontal
            ? effectivePosition === 'top'
              ? 'column'
              : 'column-reverse'
            : effectivePosition === 'right'
              ? 'row-reverse'
              : 'row',
        } as CSSProperties
      }
    >
      <AppBar
        title={title}
        primaryActions={primaryActions}
        actions={actions}
        onBack={onBack}
        sidebarPosition={effectivePosition}
      />
      {isNarrowViewport ? (
        <MobileBottomNav />
      ) : isHorizontal ? (
        <TopFunctionBar sidebarPosition={effectivePosition} />
      ) : (
        <SideNavigation />
      )}
      <div
        className={styles.main}
        style={{
          paddingTop: isTop ? FUNCTION_BAR_HEIGHT : 0,
          paddingBottom: effectivePosition === 'bottom' ? FUNCTION_BAR_HEIGHT : 0,
        }}
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
