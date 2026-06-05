import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { AppBar } from './AppBar';

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
}

export function AppShell({ children, title, actions, onBack }: AppShellProps) {
  return (
    <div className={styles.appShell}>
      <SideNavigation />
      <div className={styles.main}>
        <AppBar title={title} actions={actions} onBack={onBack} />
        <main className={styles.content}>{children}</main>
      </div>
    </div>
  );
}
