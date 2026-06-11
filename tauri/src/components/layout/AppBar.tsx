import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './AppBar.module.css';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
  titleBarOffset?: number;
}

export function AppBar({ title, actions, onBack, titleBarOffset = 0 }: AppBarProps) {
  const { t } = useTranslation('common');
  const isMac = /Mac/i.test(navigator.platform);

  return (
    <header
      className={styles.appBar}
      data-tauri-drag-region
      style={{ paddingLeft: isMac ? 80 : 20, top: titleBarOffset }}
    >
      <div className={styles.left}>
        {onBack && (
          <button className={styles.backButton} onClick={onBack} aria-label={t('back')}>
            <ArrowLeft size={20} />
          </button>
        )}
        <h1 className={styles.title}>{title}</h1>
      </div>
      <div className={styles.actions}>
        {actions}
      </div>
    </header>
  );
}
