import { ArrowLeft, LockKeyhole, UserRound } from 'lucide-react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { ToolbarActions } from '@/components/layout/ToolbarActions';
import { useAuthStore } from '@/stores/authStore';
import type { ReactNode } from 'react';

export function AndroidAppBar({
  title,
  actions,
  primaryActions,
  onBack,
}: {
  title: string;
  actions?: ReactNode;
  primaryActions?: ReactNode;
  onBack?: () => void;
}) {
  const { t } = useTranslation(['navigation', 'common']);
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const lock = useAuthStore((s) => s.lock);
  const rootPage = ['/', '/workspace', '/tools', '/settings'].includes(pathname);
  return (
    <header
      data-appbar
      data-tauri-drag-region="false"
      className="android-appbar android-glass-surface"
    >
      <div className="android-appbar-leading">
        {onBack && !rootPage ? (
          <button
            type="button"
            className="android-icon-button"
            aria-label={t('common:back')}
            onClick={onBack}
          >
            <ArrowLeft size={24} />
          </button>
        ) : (
          <ShieldLogo size={32} />
        )}
        <h1>{pathname === '/' ? 'SoloSoul' : title}</h1>
      </div>
      <div className="android-appbar-actions">
        {!primaryActions && (
          <button
            type="button"
            className="android-icon-button"
            aria-label={t('lock_vault')}
            onClick={() => void lock()}
          >
            <LockKeyhole size={24} />
          </button>
        )}
        {pathname === '/' && (
          <button
            type="button"
            className="android-icon-button"
            aria-label={t('common:material.account')}
            onClick={() => navigate('/settings/account')}
          >
            <UserRound size={24} />
          </button>
        )}
        <ToolbarActions primary={primaryActions}>{actions}</ToolbarActions>
      </div>
    </header>
  );
}
