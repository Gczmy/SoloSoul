import { useCallback } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import styles from './SideNavigation.module.css';
import { useSettingsStore } from '@/stores/settingsStore';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { PrimaryNavZone } from './PrimaryNavZone';
import { SecondaryActionBar } from './SecondaryActionBar';
import { NavButton } from './NavButton';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { ICON_SIZE } from '@/lib/constants';

export { RenameableNavButton } from './RenameableNavButton';
export { AddPageButton } from './AddPageButton';

// =============================================================================
// SideNavigation — main sidebar component
// =============================================================================

export function SideNavigation() {
  const navigate = useNavigate();
  const location = useLocation();
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('navigation');
  const vaultLock = useAuthStore((s) => s.lock);

  const handleLock = useCallback(() => vaultLock(), [vaultLock]);

  const navStyle: React.CSSProperties = isHorizontal
    ? {
        width: '100%',
        height: 48,
        flexDirection: 'row',
        borderRight: 'none',
        borderLeft: 'none',
        borderBottom: sidebarPosition === 'top' ? '1px solid var(--border-subtle)' : 'none',
        borderTop: sidebarPosition === 'bottom' ? '1px solid var(--border-subtle)' : 'none',
        padding: '0 12px',
        overflow: 'visible',
      }
    : {
        width: 48,
        height: '100vh',
        flexDirection: 'column',
        borderRight: sidebarPosition === 'left' ? '1px solid var(--border-subtle)' : 'none',
        borderLeft: sidebarPosition === 'right' ? '1px solid var(--border-subtle)' : 'none',
        borderBottom: 'none',
        borderTop: 'none',
        padding: '12px 0',
      };

  return (
    <nav className={styles.sideNav} aria-label={t('home')} style={navStyle}>
      <ShieldLogo
        size={ICON_SIZE['3xl']}
        style={isHorizontal ? { marginBottom: 0, marginRight: 12 } : { marginBottom: 16 }}
      />

      <PrimaryNavZone sidebarPosition={sidebarPosition} isHorizontal={isHorizontal} />

      {/* Foldable function button area */}
      <SecondaryActionBar sidebarPosition={sidebarPosition} isHorizontal={isHorizontal} />

      {/* Lock — always fixed, outside foldable area */}
      <NavButton
        Icon={PAGE_ICON_MAP.lock}
        label={t('lock_vault')}
        onClick={handleLock}
        position={sidebarPosition}
      />

      {/* Settings — always fixed at the bottom */}
      <NavButton
        path="/settings"
        Icon={PAGE_ICON_MAP.settings}
        label={t('settings')}
        isActive={location.pathname.startsWith('/settings')}
        onClick={() => navigate('/settings')}
        position={sidebarPosition}
      />
    </nav>
  );
}
