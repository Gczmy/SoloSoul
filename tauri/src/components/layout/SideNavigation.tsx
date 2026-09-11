import { useCallback } from 'react';
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { DesktopSidebarContext } from './DesktopSidebarContext';
import { useUiStore } from '@/stores/uiStore';
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
  const expanded = useUiStore((s) => s.sidebarExpanded) && !isHorizontal;
  const toggleExpanded = useUiStore((s) => s.toggleSidebarExpanded);
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
        width: expanded ? 232 : 48,
        height: '100vh',
        flexDirection: 'column',
        borderRight: sidebarPosition === 'left' ? '1px solid var(--border-subtle)' : 'none',
        borderLeft: sidebarPosition === 'right' ? '1px solid var(--border-subtle)' : 'none',
        borderBottom: 'none',
        borderTop: 'none',
        padding: expanded ? '12px 10px' : '12px 0',
      };

  return (
    <DesktopSidebarContext.Provider value={expanded}>
      <nav
        id="desktop-navigation"
        className={styles.sideNav}
        aria-label={t('home')}
        style={navStyle}
        data-expanded={expanded}
      >
        <div className={styles.brandHeader}>
          <ShieldLogo size={expanded ? 26 : ICON_SIZE['3xl']} />
          {expanded && <span className={styles.brandName}>SoloSoul</span>}
          <button
            type="button"
            className={styles.sidebarToggle}
            onClick={toggleExpanded}
            aria-label={t(expanded ? 'sidebar_collapse' : 'sidebar_expand')}
            aria-expanded={expanded}
            aria-controls="desktop-navigation"
          >
            {expanded ? <PanelLeftClose size={18} /> : <PanelLeftOpen size={18} />}
          </button>
        </div>

        <PrimaryNavZone sidebarPosition={sidebarPosition} isHorizontal={isHorizontal} />

        {/* Foldable function button area */}
        <SecondaryActionBar sidebarPosition={sidebarPosition} isHorizontal={isHorizontal} />

        <div className={styles.fixedActions}>
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
        </div>
      </nav>
    </DesktopSidebarContext.Provider>
  );
}
