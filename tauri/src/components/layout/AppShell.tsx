import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { TopFunctionBar } from './TopFunctionBar';
import { MobileBottomNav } from './MobileBottomNav';
import { AppBar } from './AppBar';
import { useSettingsStore } from '@/stores/settingsStore';
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
  // 窄视口下强制使用底部导航栏
  const effectivePosition = isNarrowViewport ? 'bottom' : sidebarPosition;
  const isTop = effectivePosition === 'top';
  const isHorizontal = isTop || effectivePosition === 'bottom';

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
      <AppBar
        title={title}
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
        style={
          isTop
            ? { paddingTop: FUNCTION_BAR_HEIGHT }
            : effectivePosition === 'bottom'
              ? { paddingBottom: FUNCTION_BAR_HEIGHT }
              : undefined
        }
      >
        <main className={styles.content}>{children}</main>
      </div>
    </div>
  );
}
