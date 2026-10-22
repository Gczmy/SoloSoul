import { useAndroidGlassSurface } from '@/hooks/useAndroidGlass';
import { lazy, Suspense, useEffect, useLayoutEffect, useRef, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { trackAsyncListener } from '@/lib/asyncListener';
import { useLocation } from 'react-router-dom';
import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { DesktopPagePane } from './DesktopPagePane';
import { PAGE_PANE_MIN, usePagePaneResize } from './usePagePaneResize';
import { usePagePaneMotion } from './usePagePaneMotion';
import { TopFunctionBar } from './TopFunctionBar';
import { MobileBottomNav } from './MobileBottomNav';
import { AppBar } from './AppBar';
import { PaneToggleButton } from './PaneToggleButton';
import { PairingDialog } from '@/components/sync/PairingDialog';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import { useSyncStore } from '@/stores/syncStore';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { isAndroidSync, isMobilePlatformSync } from '@/lib/platform';
import { useToastError } from '@/hooks/useToastError';
import { AndroidNavigation } from '@/components/android/AndroidNavigation';
import { ShellNotificationSlot } from './ShellNotifications';
import { DESKTOP_SIDEBAR_COLLAPSED_WIDTH, DESKTOP_SIDEBAR_EXPANDED_WIDTH } from '@/lib/constants';
import { PanelRightOpen, PanelRightClose } from 'lucide-react';
import { useMediaPreference } from '@/hooks/useMediaPreference';
import type { ObjectInspectorContext } from './shellConfigStore';

const ObjectInspectorPanel = lazy(() =>
  import('@/components/object/ObjectInspectorPanel').then((module) => ({
    default: module.ObjectInspectorPanel,
  })),
);

const FUNCTION_BAR_HEIGHT = 48;
const SHELL_LAYOUT_KEYS = [
  '--sidebar-width',
  '--shell-chrome-bottom',
  '--shell-navigation-bottom',
  '--shell-page-padding',
  ...['top', 'bottom', 'left', 'right', 'height'].map((edge) => `--shell-content-${edge}`),
];

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  primaryActions?: React.ReactNode;
  windowsContentToolbar?: boolean;
  onBack?: () => void;
  objectInspector?: ObjectInspectorContext;
}

export function AppShell({
  children,
  title,
  actions,
  primaryActions,
  windowsContentToolbar,
  onBack,
  objectInspector,
}: AppShellProps) {
  const { t } = useTranslation('settings');
  const { onError } = useToastError();
  const isAndroid = isAndroidSync();
  const isNarrowViewport = useIsNarrowViewport();
  const titlebarHeight = useNativeWindowStore((s) => s.titlebarHeight);
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const isWindows = useNativeWindowStore((s) => s.isWindows);
  const windowsShell = useNativeWindowStore((s) => Boolean(s.windowsFrame));
  const trafficLightsRight = useNativeWindowStore((s) => s.trafficLightsRight);
  const appbarHeight = isAndroid
    ? 64
    : windowsShell
      ? 52
      : isNarrowViewport
        ? 48
        : isMacOS
          ? // 全屏切换中的原生32pt临时几何不能压缩网页顶部栏。
            Math.max(52, titlebarHeight)
          : isWindows
            ? Math.max(52, titlebarHeight)
            : 48;
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const storedPaneOpen = useUiStore((s) => s.desktopPagePaneOpen);
  const inspectorRequested = useUiStore((s) => s.desktopObjectInspectorOpen);
  const setInspectorOpen = useUiStore((s) => s.setDesktopObjectInspectorOpen);
  const showPagePane = useUiStore((s) => s.showDesktopPagePane);
  const togglePagePane = useUiStore((s) => s.toggleDesktopPagePane);
  const compactDesktop = useMediaPreference('(max-width: 1099px)');
  const legacySidebarExpanded = useUiStore((s) => s.sidebarExpanded);
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
  const desktopPane = !isMobilePlatformSync() && !isNarrowViewport && !isHorizontal;
  const inspectorAvailable = desktopPane && Boolean(objectInspector);
  const inspectorOpen = inspectorAvailable && inspectorRequested;
  const paneSuppressed = inspectorOpen && compactDesktop;
  const paneOpen = storedPaneOpen && !paneSuppressed;
  const bodyRef = useRef<HTMLDivElement>(null);
  const paneResize = usePagePaneResize(bodyRef, desktopPane, paneOpen, inspectorOpen ? 340 : 0);
  const paneAnimating = usePagePaneMotion(bodyRef, desktopPane, paneOpen);
  const inspectorAnimating = usePagePaneMotion(bodyRef, desktopPane, inspectorOpen);
  const closeInspector = () => {
    setInspectorOpen(false);
    document.querySelector<HTMLButtonElement>('[data-object-inspector-toggle]')?.focus();
  };
  const InspectorIcon = inspectorOpen ? PanelRightClose : PanelRightOpen;
  const inspectorToggle = inspectorAvailable ? (
    <PaneToggleButton
      tooltip={t('common:toggle_object_inspector')}
      tooltipMarker="object-inspector"
      align="end"
      data-object-inspector-toggle
      data-ui-icon-button="neutral"
      data-ui-icon-intent="neutral"
      data-desktop-control="icon"
      aria-label={t('common:toggle_object_inspector')}
      aria-expanded={inspectorOpen}
      aria-controls="desktop-object-inspector"
      onClick={() => setInspectorOpen(!inspectorOpen)}
    >
      <InspectorIcon size={20} />
    </PaneToggleButton>
  ) : null;
  const editorActions = inspectorToggle ? (
    <>
      {primaryActions}
      {inspectorToggle}
    </>
  ) : (
    primaryActions
  );
  const contentToolbar = Boolean(
    isWindows && desktopPane && windowsContentToolbar && editorActions,
  );
  // 旧导航保留交通灯避让；macOS/Windows 页面树布局固定使用 56px 图标栏。
  const collapsedWidth = Math.max(
    DESKTOP_SIDEBAR_COLLAPSED_WIDTH,
    isMacOS && effectivePosition === 'left' ? Math.ceil(trafficLightsRight + 16) : 0,
  );
  const sidebarWidth = isAndroid
    ? 88
    : desktopPane
      ? isMacOS || isWindows
        ? 56
        : appbarHeight
      : legacySidebarExpanded
        ? Math.max(DESKTOP_SIDEBAR_EXPANDED_WIDTH, collapsedWidth)
        : collapsedWidth;

  // 导航避让与正文边界分开：通知出现不能移动交通灯/AppBar 的材质分界。
  useLayoutEffect(() => {
    const root = document.documentElement;
    // 切换宽度只更新已有几何；卸载外壳时才释放，避免中途落回默认值。
    return () => SHELL_LAYOUT_KEYS.forEach((key) => root.style.removeProperty(key));
  }, []);
  useLayoutEffect(() => {
    const root = document.documentElement;
    const values: Record<string, string> = {
      '--sidebar-width': `${sidebarWidth}px`,
      '--shell-chrome-bottom':
        isAndroid || isNarrowViewport
          ? `calc(${appbarHeight}px + var(--safe-area-top))`
          : `${appbarHeight + (isTop ? FUNCTION_BAR_HEIGHT : 0)}px`,
      '--shell-navigation-bottom': isAndroid
        ? isNarrowViewport
          ? 'calc(86px + var(--safe-area-bottom))'
          : 'var(--safe-area-bottom)'
        : isNarrowViewport
          ? 'calc(56px + var(--safe-area-bottom))'
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
    const peer = s.incomingPairingRequest;
    if (!peer) return;
    // P103: 入站配对确认时绑定握手认证指纹（B 侧配对请求来自握手认证值）
    try {
      await s.trustPeer(peer.id, true, peer.fingerprint || undefined);
      await s.loadStatus();
      s.clearIncomingPairingRequest();
    } catch (error) {
      if (useSyncStore.getState().incomingPairingRequest?.id === peer.id)
        onError(error, t('sync_pairing_title'));
    }
  };

  const handleIncomingIgnore = () => {
    useSyncStore.getState().clearIncomingPairingRequest();
  };

  return (
    <div
      className={styles.appShell}
      data-android-shell={isAndroid || undefined}
      data-macos-shell={(isMacOS && !isNarrowViewport && !isAndroid) || undefined}
      data-windows-shell={windowsShell || undefined}
      data-navigation={effectivePosition}
      data-page-pane-animating={(desktopPane && paneAnimating) || undefined}
      style={
        {
          '--appbar-height': `${appbarHeight}px`,
          // 与图文布局同次提交；子组件测量时不能继承上一状态的全局宽度。
          '--sidebar-width': `${sidebarWidth}px`,
          '--page-pane-width': `${paneResize.width}px`,
          '--object-inspector-width': '340px',
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
        primaryActions={contentToolbar ? undefined : editorActions}
        actions={actions}
        onBack={onBack}
        sidebarPosition={effectivePosition}
        pagePane={desktopPane}
        pagePaneOpen={paneOpen}
        onPagePaneToggle={() => (paneSuppressed ? showPagePane() : togglePagePane())}
      />
      {isAndroid ? (
        <AndroidNavigation />
      ) : isNarrowViewport ? (
        <MobileBottomNav />
      ) : isHorizontal ? (
        <TopFunctionBar sidebarPosition={effectivePosition} />
      ) : (
        <SideNavigation pagePane={desktopPane} />
      )}
      <div
        className={styles.main}
        data-shell-main
        style={{
          paddingTop: 'var(--shell-chrome-bottom)',
          paddingBottom: 'var(--shell-navigation-bottom)',
        }}
      >
        <div
          className={styles.contentSurface}
          data-shell-surface
          data-page-pane-layout={desktopPane || undefined}
        >
          <ShellNotificationSlot />
          <div
            ref={bodyRef}
            className={styles.contentBody}
            data-page-pane-body={desktopPane || undefined}
            data-pane-open={paneOpen}
            data-pane-resizing={paneResize.resizing || undefined}
            data-pane-animating={paneAnimating || undefined}
            data-inspector-open={inspectorOpen}
            data-inspector-animating={inspectorAnimating || undefined}
            data-content-toolbar={contentToolbar || undefined}
          >
            {desktopPane && (
              <div className={styles.paneSlot}>
                <DesktopPagePane open={paneOpen} animating={paneAnimating} />
              </div>
            )}
            {contentToolbar && (
              <div
                className={styles.contentToolbar}
                data-editor-content-toolbar
                role="group"
                aria-label={title}
              >
                <div className={styles.contentToolbarViewport}>
                  <div className={styles.contentToolbarActions} data-editor-form-actions>
                    {primaryActions}
                  </div>
                </div>
                {inspectorToggle}
              </div>
            )}
            <main ref={contentRef} className={styles.content} data-shell-content>
              {children}
            </main>
            {desktopPane && (
              <div
                id="desktop-object-inspector"
                className={styles.inspectorSlot}
                inert={!inspectorOpen}
                aria-hidden={!inspectorOpen}
              >
                {inspectorOpen && objectInspector && (
                  <Suspense fallback={<div role="status">{t('common:loading')}</div>}>
                    <ObjectInspectorPanel context={objectInspector} onClose={closeInspector} />
                  </Suspense>
                )}
              </div>
            )}
            {desktopPane && (
              <div
                className={styles.paneSeparator}
                role="separator"
                aria-label={t('navigation:page_pane_resize')}
                aria-orientation="vertical"
                aria-controls="desktop-page-pane"
                aria-valuemin={PAGE_PANE_MIN}
                aria-valuemax={paneResize.max}
                aria-valuenow={paneResize.width}
                tabIndex={paneOpen && !paneAnimating ? 0 : -1}
                aria-disabled={paneAnimating || undefined}
                aria-hidden={!paneOpen}
                {...paneResize.separatorProps}
              />
            )}
          </div>
        </div>
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
