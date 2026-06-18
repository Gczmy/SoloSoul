import { useTranslation } from 'react-i18next';
import styles from './SideNavigation.module.css';
import { useSettingsStore } from '@/stores/settingsStore';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { PrimaryNavZone } from './PrimaryNavZone';
import { SecondaryActionBar } from './SecondaryActionBar';
export { RenameableNavButton } from './RenameableNavButton';
export { AddPageButton } from './AddPageButton';

// =============================================================================
// SideNavigation — main sidebar component
// =============================================================================

export function SideNavigation() {
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('navigation');

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
        size={32}
        style={isHorizontal ? { marginBottom: 0, marginRight: 12 } : { marginBottom: 16 }}
      />

      <PrimaryNavZone sidebarPosition={sidebarPosition} isHorizontal={isHorizontal} />

      <SecondaryActionBar sidebarPosition={sidebarPosition} isHorizontal={isHorizontal} />
    </nav>
  );
}
