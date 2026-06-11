import { ArrowLeft, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import styles from './AppBar.module.css';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
  titleBarOffset?: number;
}

export function AppBar({ title, actions, onBack, titleBarOffset = 0 }: AppBarProps) {
  const { t } = useTranslation('common');
  const navigate = useNavigate();
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
        <button
          className={styles.backButton}
          onClick={() => navigate('/search')}
          aria-label={t('search')}
          title={t('search')}
          style={{ marginRight: 4 }}
        >
          <Search size={18} />
        </button>
        {actions}
      </div>
    </header>
  );
}
