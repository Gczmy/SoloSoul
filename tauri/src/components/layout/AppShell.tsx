import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { TopFunctionBar } from './TopFunctionBar';
import { MobileBottomNav } from './MobileBottomNav';
import { AppBar } from './AppBar';
import { useSettingsStore } from '@/stores/settingsStore';
import { useIsMobile } from '@/hooks/useIsMobile';

const FUNCTION_BAR_HEIGHT = 48;

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
}

export function AppShell({ children, title, actions, onBack }: AppShellProps) {
  const isMobile = useIsMobile();
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  // 移动端强制使用底部导航栏
  const effectivePosition = isMobile ? 'bottom' : sidebarPosition;
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
      {isMobile ? (
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
        <AppBar
          title={title}
          actions={actions}
          onBack={onBack}
          topBarHeight={isTop ? FUNCTION_BAR_HEIGHT : 0}
          sidebarPosition={effectivePosition}
        />
        <main className={styles.content}>{children}</main>
      </div>
    </div>
  );
}
