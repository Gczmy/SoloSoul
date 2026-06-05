import { ArrowLeft } from 'lucide-react';
import styles from './AppBar.module.css';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
}

export function AppBar({ title, actions, onBack }: AppBarProps) {
  return (
    <header className={styles.appBar}>
      <div className={styles.left}>
        {onBack && (
          <button className={styles.backButton} onClick={onBack} aria-label="Back">
            <ArrowLeft size={20} />
          </button>
        )}
        <h1 className={styles.title}>{title}</h1>
      </div>
      <div className={styles.actions}>{actions}</div>
    </header>
  );
}
