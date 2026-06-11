import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { TopFunctionBar } from './TopFunctionBar';
import { AppBar } from './AppBar';
import { useSettingsStore } from '@/stores/settingsStore';

const FUNCTION_BAR_HEIGHT = 48;

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
}

export function AppShell({ children, title, actions, onBack }: AppShellProps) {
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const isTop = sidebarPosition === 'top';
  const isHorizontal = isTop || sidebarPosition === 'bottom';

  return (
    <div
      className={styles.appShell}
      style={{
        flexDirection: isHorizontal
          ? (sidebarPosition === 'top' ? 'column' : 'column-reverse')
          : (sidebarPosition === 'right' ? 'row-reverse' : 'row'),
      }}
    >
      {isTop ? <TopFunctionBar /> : <SideNavigation />}
      <div
        className={styles.main}
        style={isTop ? { paddingTop: FUNCTION_BAR_HEIGHT } : undefined}
      >
        <AppBar title={title} actions={actions} onBack={onBack} topBarHeight={isTop ? FUNCTION_BAR_HEIGHT : 0} sidebarPosition={sidebarPosition} />
        <main className={styles.content}>{children}</main>
      </div>
    </div>
  );
}
