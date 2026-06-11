import { useState } from 'react';
import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import styles from './TitleBar.module.css';
import { SearchPopover } from './SearchPopover';

export function TitleBar() {
  const { t } = useTranslation('common');
  const [showPopover, setShowPopover] = useState(false);

  const handleMouseDown = (e: React.MouseEvent) => {
    // Only drag if clicking on the title bar itself, not on interactive elements
    if (e.target === e.currentTarget) {
      getCurrentWebviewWindow().startDragging().catch(() => {});
    }
  };

  return (
    <>
      <div className={styles.titleBar} onMouseDown={handleMouseDown}>
        <button
          className={styles.searchBar}
          onClick={() => setShowPopover(true)}
          aria-label={t('search')}
        >
          <Search size={12} />
          <span style={{ flex: 1, textAlign: 'left', overflow: 'hidden', textOverflow: 'ellipsis' }}>{t('search_bar_hint')}</span>
        </button>
      </div>
      {showPopover && <SearchPopover onClose={() => setShowPopover(false)} />}
    </>
  );
}
