import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './AppBar.module.css';
import { ToolbarActions } from './ToolbarActions';
import { ICON_SIZE } from '@/lib/constants';
import { useLayoutEffect, useRef, type CSSProperties } from 'react';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { observeTitlebarControls } from '@/lib/nativeTitlebarControls';
import { isAndroidSync } from '@/lib/platform';
import { AndroidAppBar } from '@/components/android/AndroidAppBar';

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
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
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
        <ToolbarActions primary={primaryActions}>{actions}</ToolbarActions>
      </div>
    </header>
  );
}
