import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './AppBar.module.css';
import { ICON_SIZE } from '@/lib/iconSizes';


interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
  titleBarOffset?: number;
  topBarHeight?: number;
  sidebarPosition?: 'left' | 'right' | 'top' | 'bottom';
}

export function AppBar({
  title,
  actions,
  onBack,
  titleBarOffset = 0,
  topBarHeight = 0,
  sidebarPosition = 'left',
}: AppBarProps) {
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('common');

  return (
    <header
      className={styles.appBar}
      style={{
        paddingLeft: 20,
        top: titleBarOffset + topBarHeight,
        left: isHorizontal ? 0 : sidebarPosition === 'right' ? 0 : 48,
        right: isHorizontal ? 0 : sidebarPosition === 'right' ? 48 : 0,
      }}
    >
      <div className={styles.left}>
        {onBack && (
          <button className={styles.backButton} onClick={onBack} aria-label={t('back')}>
            <ArrowLeft size={ICON_SIZE.xl} />
          </button>
        )}
        <h1 className={styles.title}>{title}</h1>
      </div>
      <div className={styles.actions}>{actions}</div>
    </header>
  );
}
