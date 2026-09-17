import { useAndroidGlassSurface } from '@/hooks/useAndroidGlass';
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
import { isAndroidSync } from '@/lib/platform';
import { AndroidNavigation } from '@/components/android/AndroidNavigation';
import { ShellNotificationSlot } from './ShellNotifications';

const FUNCTION_BAR_HEIGHT = 48;

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  primaryActions?: React.ReactNode;
  onBack?: () => void;
}

export function AppShell({ children, title, actions, primaryActions, onBack }: AppShellProps) {
  const isAndroid = isAndroidSync();
  const isNarrowViewport = useIsNarrowViewport();
  const titlebarHeight = useNativeWindowStore((s) => s.titlebarHeight);
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const isWindows = useNativeWindowStore((s) => s.isWindows);
  const trafficLightsRight = useNativeWindowStore((s) => s.trafficLightsRight);
  const appbarHeight = isAndroid
    ? 64
    : isNarrowViewport
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
  const effectivePosition = isAndroid
    ? isNarrowViewport
      ? 'bottom'
      : 'left'
    : isNarrowViewport
      ? 'bottom'
      : sidebarPosition;
  const isTop = effectivePosition === 'top';
  const isHorizontal = isTop || effectivePosition === 'bottom';
  // 桌面折叠栏统一为 96px 图文轨道；macOS 左侧按交通灯实际宽度继续扩展。
  const collapsedWidth = Math.max(
    96,
    isMacOS && effectivePosition === 'left' ? Math.ceil(trafficLightsRight + 16) : 0,
  );
  const sidebarWidth = isAndroid
    ? 88
    : sidebarExpanded
      ? Math.max(232, collapsedWidth)
      : collapsedWidth;

  // 导航避让与正文边界分开：通知出现不能移动交通灯/AppBar 的材质分界。
  useLayoutEffect(() => {
    const root = document.documentElement;
    const values: Record<string, string> = {
      '--sidebar-width': `${sidebarWidth}px`,
      '--shell-chrome-bottom':
        isAndroid || isNarrowViewport
          ? `calc(${appbarHeight}px + env(safe-area-inset-top, 0px))`
          : `${appbarHeight + (isTop ? FUNCTION_BAR_HEIGHT : 0)}px`,
      '--shell-navigation-bottom': isAndroid
        ? isNarrowViewport
          ? 'calc(86px + env(safe-area-inset-bottom, 0px))'
          : 'env(safe-area-inset-bottom, 0px)'
        : isNarrowViewport
          ? 'calc(56px + env(safe-area-inset-bottom, 0px))'
          : `${effectivePosition === 'bottom' ? FUNCTION_BAR_HEIGHT : 0}px`,
      '--shell-page-padding': isNarrowViewport ? '16px' : '24px',
    };
    Object.entries(values).forEach(([key, value]) => root.style.setProperty(key, value));
    // 固定聊天面板、尺标及高度受限页面读取真实正文边界（包含通知实际高度）。
    // 发布像素值，兼容 objectRuler 对这些变量的数值读取；不对内容加 transform。
    const content = contentRef.current;
    const geometryKeys = ['top', 'bottom', 'left', 'right', 'height'].map(
      (edge) => `--shell-content-${edge}`,
    );
    const measure = () => {
      if (!content) return;
      const rect = content.getBoundingClientRect();
      const geometry = [
        rect.top,
        window.innerHeight - rect.bottom,
        rect.left,
        window.innerWidth - rect.right,
        rect.height,
      ];
      geometryKeys.forEach((key, index) => {
        const value = `${Math.max(0, geometry[index])}px`;
        if (root.style.getPropertyValue(key) !== value) root.style.setProperty(key, value);
      });
    };
    measure();
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure);
    if (content) observer?.observe(content);
    window.addEventListener('resize', measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener('resize', measure);
      [...Object.keys(values), ...geometryKeys].forEach((key) => root.style.removeProperty(key));
    };
  }, [
    sidebarWidth,
    effectivePosition,
    isHorizontal,
    isTop,
    isNarrowViewport,
    appbarHeight,
    isAndroid,
  ]);

  useAndroidGlassSurface();
  const reduceMotion = useSettingsStore((s) => s.settings.reduceMotion);
  useLayoutEffect(() => {
    document.documentElement.dataset.userReduceMotion = String(reduceMotion);
  }, [reduceMotion]);

  useEffect(() => {
    if (!isAndroid || !window.visualViewport) return;
    const viewport = window.visualViewport;
    const update = () => {
      const inset = Math.max(0, window.innerHeight - viewport.height - viewport.offsetTop);
      document.documentElement.style.setProperty('--android-keyboard-inset', `${inset}px`);
    };
    update();
    viewport.addEventListener('resize', update);
    viewport.addEventListener('scroll', update);
    return () => {
      viewport.removeEventListener('resize', update);
      viewport.removeEventListener('scroll', update);
      document.documentElement.style.removeProperty('--android-keyboard-inset');
    };
  }, [isAndroid]);

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
      data-android-shell={isAndroid || undefined}
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
      {isAndroid ? (
        <AndroidNavigation />
      ) : isNarrowViewport ? (
        <MobileBottomNav />
      ) : isHorizontal ? (
        <TopFunctionBar sidebarPosition={effectivePosition} />
      ) : (
        <SideNavigation />
      )}
      <div
        className={styles.main}
        data-shell-main
        style={{
          paddingTop: 'var(--shell-chrome-bottom)',
          paddingBottom: 'var(--shell-navigation-bottom)',
        }}
      >
        <ShellNotificationSlot />
        <main ref={contentRef} className={styles.content} data-shell-content>
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
