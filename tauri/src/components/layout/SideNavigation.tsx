import { useLocation, useNavigate } from 'react-router-dom';
import { Home, User, Briefcase, Wallet, Lock, Search, Settings } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import styles from './SideNavigation.module.css';
import { useVaultStore } from '@/stores/vaultStore';

interface NavLink {
  type: 'link';
  path: string;
  icon: LucideIcon;
  label: string;
}

interface NavAction {
  type: 'action';
  icon: LucideIcon;
  label: string;
  action: () => void;
}

type NavItem = NavLink | NavAction;

const primaryItems: NavLink[] = [
  { type: 'link', path: '/', icon: Home, label: 'Home' },
  { type: 'link', path: '/workspace', icon: User, label: 'Profile' },
  { type: 'link', path: '/workspace?section=travel', icon: Briefcase, label: 'Travel' },
  { type: 'link', path: '/workspace?section=financial', icon: Wallet, label: 'Financial' },
];

const secondaryItems: NavItem[] = [
  { type: 'action', icon: Lock, label: 'Lock Vault', action: () => {} },
  { type: 'link', path: '/search', icon: Search, label: 'Search' },
  { type: 'link', path: '/settings', icon: Settings, label: 'Settings' },
];

function NavButton({
  path,
  Icon,
  label,
  isActive,
  onClick,
}: {
  path?: string;
  Icon: LucideIcon;
  label: string;
  isActive?: boolean;
  onClick: () => void;
}) {
  return (
    <div className={styles.navItemWrapper}>
      {path && (
        <div className={`${styles.activeIndicator} ${isActive ? styles.activeIndicatorVisible : ''}`} />
      )}
      <button
        className={`${styles.navButton} ${isActive ? styles.activeButton : ''}`}
        onClick={onClick}
        title={label}
      >
        <Icon size={20} />
      </button>
      <div className={styles.nameCard} aria-hidden="true">
        {label}
      </div>
    </div>
  );
}

export function SideNavigation() {
  const navigate = useNavigate();
  const location = useLocation();
  const vaultLock = useVaultStore((s) => s.lock);

  const items = secondaryItems.map((item) =>
    item.type === 'action' ? { ...item, action: vaultLock } as NavAction : item
  );

  return (
    <nav className={styles.sideNav}>
      <div className={styles.logo}>S</div>

      <div className={styles.navPrimary}>
        {primaryItems.map((item) => {
          const isActive =
            item.path === '/'
              ? location.pathname === '/'
              : location.pathname.startsWith('/workspace') &&
                (item.path === '/workspace'
                  ? !location.search.includes('section=')
                  : location.search.includes(item.path.split('section=')[1]));
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={item.icon}
              label={item.label}
              isActive={isActive}
              onClick={() => navigate(item.path)}
            />
          );
        })}
      </div>

      <div className={styles.navSecondary}>
        {items.map((item, i) => {
          if (item.type === 'action') {
            return (
              <NavButton
                key={`action-${i}`}
                Icon={item.icon}
                label={item.label}
                onClick={item.action}
              />
            );
          }
          const isActive = location.pathname.startsWith(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={item.icon}
              label={item.label}
              isActive={isActive}
              onClick={() => navigate(item.path)}
            />
          );
        })}
      </div>
    </nav>
  );
}
