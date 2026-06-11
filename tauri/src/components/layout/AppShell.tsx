import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { AppBar } from './AppBar';
import { useSettingsStore } from '@/stores/settingsStore';


interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
}

export function AppShell({ children, title, actions, onBack }: AppShellProps) {
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const titleBarOffset = /Mac/i.test(navigator.platform) ? 28 : 0;

  return (
    <div
      className={styles.appShell}
      style={{
        flexDirection: isHorizontal
          ? (sidebarPosition === 'top' ? 'column' : 'column-reverse')
          : (sidebarPosition === 'right' ? 'row-reverse' : 'row'),
        paddingTop: titleBarOffset,
      }}
    >
      <SideNavigation titleBarOffset={titleBarOffset} />
      <div className={styles.main}>
        <AppBar title={title} actions={actions} onBack={onBack} titleBarOffset={titleBarOffset} />
        <main className={styles.content}>{children}</main>
      </div>
    </div>
  );
}
