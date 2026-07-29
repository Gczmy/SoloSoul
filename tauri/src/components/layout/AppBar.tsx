import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './AppBar.module.css';
import { ICON_SIZE } from '@/lib/constants';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
  sidebarPosition?: 'left' | 'right' | 'top' | 'bottom';
}

export function AppBar({ title, actions, onBack, sidebarPosition = 'left' }: AppBarProps) {
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('common');

  return (
    <header
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
      <div className={styles.actions}>{actions}</div>
    </header>
  );
}
