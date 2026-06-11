import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import styles from './TitleBar.module.css';

export function TitleBar() {
  const { t } = useTranslation('common');
  const navigate = useNavigate();

  return (
    <div className={styles.titleBar} data-tauri-drag-region>
      <button
        className={styles.searchBar}
        onClick={() => navigate('/search')}
        aria-label={t('search')}
      >
        <Search size={12} />
        <span>{t('search')}</span>
      </button>
    </div>
  );
}
