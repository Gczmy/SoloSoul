import { useLocation, useNavigate } from 'react-router-dom';
import { Home, User, Briefcase, Wallet, Search, Settings } from 'lucide-react';
import styles from './SideNavigation.module.css';

/* 9.2 — Primary Nav（上区）：核心数据对象对应的默认页面 */
const primaryItems = [
  { path: '/', icon: Home, label: 'Home' },
  { path: '/workspace', icon: User, label: 'Profile' },
  { path: '/workspace?section=travel', icon: Briefcase, label: 'Travel' },
  { path: '/workspace?section=financial', icon: Wallet, label: 'Financial' },
];

/* 9.2 — Secondary Nav（下区）：工具性功能入口 */
const secondaryItems = [
  { path: '/search', icon: Search, label: 'Search' },
  { path: '/settings', icon: Settings, label: 'Settings' },
];

function NavButton({
  path,
  Icon,
  label,
  isActive,
  navigate,
}: {
  path: string;
  Icon: React.ComponentType<{ size: number }>;
  label: string;
  isActive: boolean;
  navigate: (path: string) => void;
}) {
  return (
    <div className={styles.navItemWrapper}>
      <div className={`${styles.activeIndicator} ${isActive ? styles.activeIndicatorVisible : ''}`} />
      <button
        className={`${styles.navButton} ${isActive ? styles.activeButton : ''}`}
        onClick={() => navigate(path)}
        title={label}
      >
        <Icon size={20} />
      </button>
      {/* 9.3 — Warp 风格悬停名称卡片 */}
      <div className={styles.nameCard} aria-hidden="true">
        {label}
      </div>
    </div>
  );
}

export function SideNavigation() {
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <nav className={styles.sideNav}>
      <div className={styles.logo}>S</div>

      {/* 9.2 — Primary Nav（上区） */}
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
              navigate={navigate}
            />
          );
        })}
      </div>

      {/* 9.2 — Secondary Nav（下区） */}
      <div className={styles.navSecondary}>
        {secondaryItems.map((item) => {
          const isActive = location.pathname.startsWith(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={item.icon}
              label={item.label}
              isActive={isActive}
              navigate={navigate}
            />
          );
        })}
      </div>
    </nav>
  );
}
