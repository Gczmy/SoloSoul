import { useCallback } from 'react';
import { DesktopSidebarContext } from './DesktopSidebarContext';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import styles from './SideNavigation.module.css';
import { useSettingsStore } from '@/stores/settingsStore';
import { PrimaryNavZone } from './PrimaryNavZone';
import { SecondaryActionBar } from './SecondaryActionBar';
import { NavButton } from './NavButton';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { DESKTOP_SIDEBAR_COLLAPSED_WIDTH } from '@/lib/constants';

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
  const expanded = false;
  const windowsShell = useNativeWindowStore((s) => Boolean(s.windowsFrame));
  const vaultLock = useAuthStore((s) => s.lock);

  const handleLock = useCallback(() => vaultLock(), [vaultLock]);

  const navStyle: React.CSSProperties = isHorizontal
    ? {
        width: '100%',
        height: 48,
        flexDirection: 'row',
        borderRight: 'none',
        borderLeft: 'none',
        borderBottom:
          sidebarPosition === 'top'
            ? '1px solid var(--navigation-divider, var(--border-subtle))'
            : 'none',
        borderTop:
          sidebarPosition === 'bottom'
            ? '1px solid var(--navigation-divider, var(--border-subtle))'
            : 'none',
        padding: '0 12px',
        overflow: 'visible',
      }
    : {
        width: `var(--sidebar-width, ${DESKTOP_SIDEBAR_COLLAPSED_WIDTH}px)`,
        height: '100vh',
        flexDirection: 'column',
        borderRight: 'none',
        borderLeft: 'none',
        borderBottom: 'none',
        borderTop: 'none',
        paddingInline: 0,
        paddingBottom: 12,
      };

  return (
    <DesktopSidebarContext.Provider value={expanded}>
      <nav
        id="desktop-navigation"
        className={styles.sideNav}
        aria-label={t('home')}
        style={navStyle}
        data-expanded={expanded}
        data-position={sidebarPosition}
        data-windows-shell={windowsShell || undefined}
        data-desktop-rail
      >
        {isHorizontal ? (
          <PrimaryNavZone sidebarPosition={sidebarPosition} isHorizontal />
        ) : (
          <div className={styles.primaryZone}>
            <PrimaryNavZone sidebarPosition={sidebarPosition} isHorizontal={false} />
          </div>
        )}

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
