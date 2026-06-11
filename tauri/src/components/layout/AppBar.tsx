import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './AppBar.module.css';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
  titleBarOffset?: number;
  topBarHeight?: number;
}

export function AppBar({ title, actions, onBack, titleBarOffset = 0, topBarHeight = 0 }: AppBarProps) {
  const { t } = useTranslation('common');

  return (
    <header
      className={styles.appBar}
      style={{
        paddingLeft: 20,
        top: titleBarOffset + topBarHeight,
        left: topBarHeight > 0 ? 0 : undefined,
        right: topBarHeight > 0 ? 0 : undefined,
      }}
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
