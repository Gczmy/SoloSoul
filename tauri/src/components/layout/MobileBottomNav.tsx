import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Home, Search, MessageSquare, Settings, Lock } from 'lucide-react';
import styles from './MobileBottomNav.module.css';
import { useVaultStore } from '@/stores/vaultStore';

const NAV_ITEMS = [
  { path: '/', labelKey: 'home', Icon: Home },
  { path: '/search', labelKey: 'search', Icon: Search },
  { path: '/llm-chat', labelKey: 'ai_chat', Icon: MessageSquare },
  { path: '/settings', labelKey: 'settings', Icon: Settings },
] as const;

export function MobileBottomNav() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('navigation');
  const lock = useVaultStore((s) => s.lock);

  return (
    <nav className={styles.bottomNav} aria-label={t('home')}>
      {NAV_ITEMS.map(({ path, labelKey, Icon }) => {
        const isActive = path === '/' ? location.pathname === '/' : location.pathname.startsWith(path);
        return (
          <button
            key={path}
            type="button"
            className={`${styles.navItem} ${isActive ? styles.navItemActive : ''}`}
            onClick={() => navigate(path)}
            aria-current={isActive ? 'page' : undefined}
          >
            <Icon size={22} />
            <span className={styles.navLabel}>{t(labelKey)}</span>
          </button>
        );
      })}
      {/* Lock vault quick action — placed as the last item */}
      <button type="button" className={styles.navItem} onClick={lock}>
        <Lock size={22} />
        <span className={styles.navLabel}>{t('lock_vault')}</span>
      </button>
    </nav>
  );
}
