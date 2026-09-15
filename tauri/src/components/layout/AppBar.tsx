import {
  ArrowLeft,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './AppBar.module.css';
import { ICON_SIZE } from '@/lib/constants';
import { useLayoutEffect, useRef, type CSSProperties } from 'react';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { observeTitlebarControls } from '@/lib/nativeTitlebarControls';
import { useUiStore } from '@/stores/uiStore';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
  sidebarPosition?: 'left' | 'right' | 'top' | 'bottom';
}

export function AppBar({ title, actions, onBack, sidebarPosition = 'left' }: AppBarProps) {
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('common');
  const { t: nav } = useTranslation('navigation');
  const isNarrow = useIsNarrowViewport();
  const expanded = useUiStore((s) => s.sidebarExpanded);
  const toggleExpanded = useUiStore((s) => s.toggleSidebarExpanded);
  const ToggleIcon =
    sidebarPosition === 'right'
      ? expanded
        ? PanelRightClose
        : PanelRightOpen
      : expanded
        ? PanelLeftClose
        : PanelLeftOpen;
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const trafficLightsRight = useNativeWindowStore((s) => s.trafficLightsRight);
  const headerRef = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    if (!isMacOS || !headerRef.current) return;
    return observeTitlebarControls(headerRef.current);
  }, [isMacOS]);
  // 右侧或横向导航时，正文左上控件仍需横向避让交通灯。
  const contentLeft = sidebarPosition === 'left' ? 'var(--sidebar-width, 48px)' : '0px';

  return (
    <header
      ref={headerRef}
      data-appbar
      data-native-compact={isMacOS || undefined}
      data-tauri-drag-region={isMacOS ? 'false' : 'deep'}
      style={
        {
          '--appbar-leading-inset': `max(12px, calc(${trafficLightsRight > 0 ? trafficLightsRight + 12 : 0}px - ${contentLeft}))`,
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
      <div className={styles.left}>
        {!isHorizontal && !isNarrow && (
          <button
            data-titlebar-control
            data-tauri-drag-region="false"
            type="button"
            className={styles.backButton}
            onClick={toggleExpanded}
            aria-label={nav(expanded ? 'sidebar_collapse' : 'sidebar_expand')}
            aria-expanded={expanded}
            aria-controls="desktop-navigation"
          >
            <ToggleIcon size={20} />
          </button>
        )}
        {onBack && (
          <button
            data-titlebar-control
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
        {actions}
      </div>
    </header>
  );
}
