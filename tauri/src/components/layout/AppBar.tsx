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
import { useLayoutEffect, useRef, type CSSProperties } from 'react';
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
}

export function AppBar({
  title,
  actions,
  primaryActions,
  onBack,
  sidebarPosition = 'left',
}: AppBarProps) {
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('common');
  const { t: navT } = useTranslation('navigation');
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const isNarrowViewport = useIsNarrowViewport();
  const sidebarExpanded = useUiStore((s) => s.sidebarExpanded);
  const toggleSidebarExpanded = useUiStore((s) => s.toggleSidebarExpanded);
  const macShell = isMacOS && !isNarrowViewport && !isAndroidSync();
  const hasSidebarToggle = macShell && !isHorizontal;
  const ToggleIcon =
    sidebarPosition === 'right'
      ? sidebarExpanded
        ? PanelRightClose
        : PanelRightOpen
      : sidebarExpanded
        ? PanelLeftClose
        : PanelLeftOpen;
  const trafficLightsRight = useNativeWindowStore((s) => s.trafficLightsRight);
  const headerRef = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    if (!isMacOS || !headerRef.current) return;
    return observeTitlebarControls(headerRef.current);
  }, [isMacOS]);
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
      data-native-compact={isMacOS || undefined}
      data-macos-shell={macShell || undefined}
      data-tauri-drag-region={isMacOS ? 'false' : 'deep'}
      style={
        {
          '--appbar-leading-inset': `max(12px, calc(${trafficLightsRight > 0 ? trafficLightsRight + 12 : 0}px - ${contentLeft}))`,
          '--mac-sidebar-control-left': `${trafficLightsRight + 12}px`,
          '--mac-page-title-left': `max(${contentLeft}, ${trafficLightsRight + (hasSidebarToggle ? 56 : 12)}px)`,
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
          type="button"
          className={styles.sidebarToggle}
          data-titlebar-control
          data-desktop-control="icon"
          data-tauri-drag-region="false"
          aria-label={navT(sidebarExpanded ? 'sidebar_collapse' : 'sidebar_expand')}
          aria-expanded={sidebarExpanded}
          aria-controls="desktop-navigation"
          onClick={toggleSidebarExpanded}
        >
          <ToggleIcon size={20} />
        </button>
      )}
      <div className={styles.left}>
        {onBack && (
          <button
            data-titlebar-control
            data-desktop-control="icon"
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
