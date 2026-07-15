import { useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Home, Settings, Lock, ChevronDown } from 'lucide-react';
import styles from './MobileBottomNav.module.css';
import { useVaultStore } from '@/stores/vaultStore';
import { AddPageButton } from './AddPageButton';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { useMobileNavActions } from './useNavigationItems';
import { ICON_SIZE } from '@/lib/constants';

const NAV_ITEMS = [
  { path: '/', labelKey: 'home', Icon: Home },
  { path: '/settings', labelKey: 'settings', Icon: Settings },
] as const;

export function MobileBottomNav() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('navigation');
  const lock = useVaultStore((s) => s.lock);
  const [expanded, setExpanded] = useState(false);

  const { items } = useMobileNavActions();

  const handleNavigate = (path: string) => {
    navigate(path);
    setExpanded(false);
  };

  const handleLock = () => {
    lock();
    setExpanded(false);
  };

  const renderPlainButton = (item: (typeof items)[number]) => {
    const Icon = PAGE_ICON_MAP[item.iconKey];
    const isLink = item.type === 'link';
    const isActive = isLink
      ? item.path === '/'
        ? location.pathname === '/'
        : location.pathname.startsWith(item.path)
      : false;
    return (
      <button
        key={isLink ? item.path : item.iconKey}
        type="button"
        className={`${styles.functionButton} ${isActive ? styles.functionButtonActive : ''}`}
        onClick={() => {
          if (isLink) {
            handleNavigate(item.path);
          } else if ('action' in item) {
            item.action();
          }
        }}
      >
        <Icon size={ICON_SIZE.xl} />
        <span className={styles.functionLabel}>{t(item.labelKey)}</span>
      </button>
    );
  };

  return (
    <>
      {/* Expandable function button panel */}
      {expanded && (
        <div className={styles.functionPanel}>
          <div className={styles.functionGrid}>
            {items.map((item) => renderPlainButton(item))}
          </div>
        </div>
      )}

      <nav className={styles.bottomNav} aria-label={t('home')}>
        {NAV_ITEMS.map(({ path, labelKey, Icon }) => {
          const isActive =
            path === '/' ? location.pathname === '/' : location.pathname.startsWith(path);
          return (
            <button
              key={path}
              type="button"
              className={`${styles.navItem} ${isActive ? styles.navItemActive : ''}`}
              onClick={() => handleNavigate(path)}
              aria-current={isActive ? 'page' : undefined}
            >
              <Icon size={22} />
              <span className={styles.navLabel}>{t(labelKey)}</span>
            </button>
          );
        })}

        {/* Add page */}
        <AddPageButton
          onCreate={(page) => handleNavigate(`/workspace/custom/${page.id}`)}
          position="bottom"
          className={styles.addPageItem}
          buttonClassName={styles.addPageTrigger}
          showLabel
          showDescription
        />

        {/* Expand / collapse toggle */}
        <button
          type="button"
          className={`${styles.navItem} ${expanded ? styles.navItemActive : ''}`}
          onClick={() => setExpanded((prev) => !prev)}
          aria-label={expanded ? t('common:collapse') : t('common:expand')}
          aria-expanded={expanded}
        >
          <ChevronDown
            size={22}
            className={`${styles.toggleIcon} ${expanded ? styles.toggleIconExpanded : ''}`}
          />
          <span className={styles.navLabel}>{expanded ? t('common:collapse') : t('common:expand')}</span>
        </button>

        {/* Lock vault quick action */}
        <button type="button" className={styles.navItem} onClick={handleLock}>
          <Lock size={22} />
          <span className={styles.navLabel}>{t('lock_vault')}</span>
        </button>
      </nav>
    </>
  );
}
