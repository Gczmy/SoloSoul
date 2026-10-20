import {
  ArrowLeft,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './AppBar.module.css';
import { ToolbarActions } from './ToolbarActions';
import { ICON_SIZE } from '@/lib/constants';
import { useEffect, useId, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import { createPortal } from 'react-dom';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import navStyles from './NavButton.module.css';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { observeTitlebarControls } from '@/lib/nativeTitlebarControls';
import { isAndroidSync } from '@/lib/platform';
import { AndroidAppBar } from '@/components/android/AndroidAppBar';
import { useUiStore } from '@/stores/uiStore';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  primaryActions?: React.ReactNode;
  onBack?: () => void;
  sidebarPosition?: 'left' | 'right' | 'top' | 'bottom';
  pagePane?: boolean;
}

export function AppBar({
  title,
  actions,
  primaryActions,
  onBack,
  sidebarPosition = 'left',
  pagePane = false,
}: AppBarProps) {
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('common');
  const { t: navT } = useTranslation('navigation');
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const isNarrowViewport = useIsNarrowViewport();
  const paneOpen = useUiStore((s) => s.desktopPagePaneOpen);
  const togglePane = useUiStore((s) => s.toggleDesktopPagePane);
  const macShell = isMacOS && !isNarrowViewport && !isAndroidSync();
  const windowsShell = useNativeWindowStore((s) => Boolean(s.windowsFrame));
  const windowsHeaderHeight = useNativeWindowStore((s) => s.windowsFrame?.headerHeight);
  const hasSidebarToggle = pagePane && !isHorizontal;
  const toggleRef = useRef<HTMLButtonElement>(null);
  const toggleHintId = useId();
  const [toggleFocused, setToggleFocused] = useState(false);
  const toggleHint = useHoverCardPosition(toggleRef, {
    isHorizontal: true,
    isBottom: false,
    isRight: false,
  });
  const toggleName = navT(paneOpen ? 'page_pane_collapse' : 'page_pane_expand');
  const showToggleHint = hasSidebarToggle && (toggleHint.isHovered || toggleFocused);
  const hideToggleHint = toggleHint.handleMouseLeave;
  useEffect(() => {
    if (!showToggleHint) return;
    const close = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setToggleFocused(false);
        hideToggleHint();
      }
    };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [showToggleHint, hideToggleHint]);
  const ToggleIcon =
    sidebarPosition === 'right'
      ? paneOpen
        ? PanelRightClose
        : PanelRightOpen
      : paneOpen
        ? PanelLeftClose
        : PanelLeftOpen;
  const trafficLightsRight = useNativeWindowStore((s) => s.trafficLightsRight);
  const headerRef = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    if ((!isMacOS && !windowsShell) || !headerRef.current) return;
    return observeTitlebarControls(headerRef.current);
  }, [isMacOS, windowsShell]);
  // 右侧或横向导航时，正文左上控件仍需横向避让交通灯。
  const contentLeft = sidebarPosition === 'left' ? 'var(--sidebar-width, 48px)' : '0px';

  if (isAndroidSync())
    return (
      <AndroidAppBar
        title={title}
        actions={actions}
        primaryActions={primaryActions}
        onBack={onBack}
      />
    );

  return (
    <header
      ref={headerRef}
      data-appbar
      data-native-compact={isMacOS || windowsShell || undefined}
      data-macos-shell={macShell || undefined}
      data-windows-shell={windowsShell || undefined}
      data-page-pane-toggle={hasSidebarToggle || undefined}
      data-tauri-drag-region={isMacOS || windowsHeaderHeight === 0 ? 'false' : 'deep'}
      style={
        {
          '--appbar-leading-inset': `max(12px, calc(${trafficLightsRight > 0 ? trafficLightsRight + 12 : 0}px - ${contentLeft}))`,
          '--mac-sidebar-control-left': `${trafficLightsRight + 12}px`,
          '--mac-page-title-left': `max(${contentLeft}, ${trafficLightsRight + (hasSidebarToggle ? 56 : 12)}px)`,
          '--windows-page-title-left': `calc(${contentLeft} + 12px)`,
          '--windows-page-trailing-inset':
            sidebarPosition === 'right' ? 'calc(var(--sidebar-width) + 12px)' : '12px',
          '--page-pane-title-offset':
            pagePane && paneOpen ? 'var(--page-pane-width, 264px)' : '0px',
        } as CSSProperties
      }
      className={[
        styles.appBar,
        isHorizontal ? styles.horizontal : styles.vertical,
        sidebarPosition === 'top' && styles.belowTopBar,
        sidebarPosition === 'right' && styles.rightSidebar,
      ]
        .filter(Boolean)
        .join(' ')}
    >
      {hasSidebarToggle && (
        <button
          ref={toggleRef}
          type="button"
          className={styles.sidebarToggle}
          data-titlebar-control
          data-desktop-control="icon"
          data-tauri-drag-region="false"
          aria-label={toggleName}
          aria-describedby={showToggleHint ? toggleHintId : undefined}
          aria-expanded={paneOpen}
          aria-controls="desktop-page-pane"
          onClick={togglePane}
          onMouseEnter={toggleHint.handleMouseEnter}
          onMouseLeave={hideToggleHint}
          onFocus={(event) => {
            if (event.currentTarget.matches(':focus-visible')) {
              setToggleFocused(true);
              toggleHint.updateCardPosition();
            }
          }}
          onBlur={() => setToggleFocused(false)}
        >
          <ToggleIcon size={20} />
        </button>
      )}
      {showToggleHint &&
        toggleHint.cardStyle &&
        createPortal(
          <div
            id={toggleHintId}
            role="tooltip"
            data-page-pane-toggle-tooltip
            data-macos-glass="tooltip"
            className={navStyles.nameCardPortalHorizontal}
            style={{
              position: 'fixed',
              ...toggleHint.cardStyle,
              left: Math.max(8, Number(toggleHint.cardStyle.left) - 20),
              transform: 'none',
              zIndex: 200,
            }}
          >
            {toggleName}
          </div>,
          document.body,
        )}
      <div className={styles.left}>
        {onBack && (
          <button
            data-titlebar-control
            data-desktop-control="icon"
            data-tauri-drag-region="false"
            type="button"
            className={styles.backButton}
            onClick={onBack}
            aria-label={t('back')}
          >
            <ArrowLeft size={ICON_SIZE.xl} />
          </button>
        )}
        <h1 className={styles.title}>{title}</h1>
      </div>
      <div className={styles.actions} data-titlebar-control data-tauri-drag-region="false">
        <ToolbarActions primary={primaryActions}>{actions}</ToolbarActions>
      </div>
    </header>
  );
}
