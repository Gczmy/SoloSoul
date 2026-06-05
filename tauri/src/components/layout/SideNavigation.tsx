import { useLocation, useNavigate } from 'react-router-dom';
import { Home, User, Wallet, Briefcase, Settings, Search, Trash2 } from 'lucide-react';
import styles from './SideNavigation.module.css';

const navItems = [
  { path: '/', icon: Home, label: 'Home' },
  { path: '/workspace', icon: User, label: 'Profile' },
  { path: '/workspace?section=travel', icon: Briefcase, label: 'Travel' },
  { path: '/workspace?section=financial', icon: Wallet, label: 'Financial' },
  { path: '/search', icon: Search, label: 'Search' },
  { path: '/settings', icon: Settings, label: 'Settings' },
];

export function SideNavigation() {
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <nav className={styles.sideNav}>
      <div className={styles.logo}>S</div>
      <div className={styles.navItems}>
        {navItems.map((item) => {
          const isActive =
            location.pathname === item.path ||
            (item.path.startsWith('/workspace') && location.pathname === '/workspace');
          const Icon = item.icon;
          return (
            <button
              key={item.path}
              className={`${styles.navItem} ${isActive ? styles.active : ''}`}
              onClick={() => navigate(item.path)}
              title={item.label}
            >
              <Icon size={20} />
            </button>
          );
        })}
      </div>
    </nav>
  );
}
