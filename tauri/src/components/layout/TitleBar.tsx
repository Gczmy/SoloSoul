import { useState } from 'react';
import { Search, X, Minus, Square } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import styles from './TitleBar.module.css';
import { SearchPopover } from './SearchPopover';

export function TitleBar() {
  const { t } = useTranslation('common');
  const [showPopover, setShowPopover] = useState(false);
  const window = getCurrentWebviewWindow();

  return (
    <>
      <div className={styles.titleBar} data-tauri-drag-region>
        {/* Traffic light buttons */}
        <div className={styles.trafficLights} data-tauri-drag-region="false">
          <button
            className={`${styles.trafficLight} ${styles.close}`}
            onClick={() => window.close()}
            aria-label="Close"
          >
            <X size={8} strokeWidth={3} />
          </button>
          <button
            className={`${styles.trafficLight} ${styles.minimize}`}
            onClick={() => window.minimize()}
            aria-label="Minimize"
          >
            <Minus size={8} strokeWidth={3} />
          </button>
          <button
            className={`${styles.trafficLight} ${styles.maximize}`}
            onClick={() => window.toggleMaximize()}
            aria-label="Maximize"
          >
            <Square size={8} strokeWidth={3} />
          </button>
        </div>

        {/* Search bar */}
        <button
          className={styles.searchBar}
          onClick={() => setShowPopover(true)}
          aria-label={t('search')}
          data-tauri-drag-region="false"
        >
          <Search size={12} />
          <span style={{ flex: 1, textAlign: 'left', overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {t('search_bar_hint')}
          </span>
        </button>
      </div>
      {showPopover && <SearchPopover onClose={() => setShowPopover(false)} />}
    </>
  );
}
