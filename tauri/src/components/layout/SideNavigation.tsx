import { useState, useRef, useCallback, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import {
  Home,
  User,
  MapPin,
  Wallet,
  Briefcase,
  FileText,
  Plus,
  Lock,
  Search,
  Puzzle,
  MessageSquare,
  Settings,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import styles from './SideNavigation.module.css';
import { useVaultStore } from '@/stores/vaultStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { useTranslation } from 'react-i18next';
import type { CustomPage } from '@/stores/settingsStore';

interface NavLink {
  type: 'link';
  path: string;
  icon: LucideIcon;
  labelKey: string; // i18n key in navigation namespace
}

interface NavAction {
  type: 'action';
  icon: LucideIcon;
  labelKey: string;
  action: () => void;
}

type NavItem = NavLink | NavAction;

const primaryItems: NavLink[] = [
  { type: 'link', path: '/', icon: Home, labelKey: 'home' },
  { type: 'link', path: '/workspace', icon: User, labelKey: 'profile' },
  { type: 'link', path: '/workspace?section=travel', icon: MapPin, labelKey: 'travel' },
  { type: 'link', path: '/workspace?section=financial', icon: Wallet, labelKey: 'financial' },
  { type: 'link', path: '/workspace?section=professional', icon: Briefcase, labelKey: 'professional' },
];

const secondaryItems: NavItem[] = [
  { type: 'action', icon: Lock, labelKey: 'lock_vault', action: () => {} },
  { type: 'link', path: '/search', icon: Search, labelKey: 'search' },
  { type: 'link', path: '/plugins', icon: Puzzle, labelKey: 'plugin' },
  { type: 'link', path: '/llm-chat', icon: MessageSquare, labelKey: 'ai_chat' },
  { type: 'link', path: '/settings', icon: Settings, labelKey: 'settings' },
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
        aria-label={label}
        aria-current={isActive ? 'page' : undefined}
      >
        <Icon size={20} />
      </button>
      <div className={styles.nameCard} role="tooltip" aria-hidden="true">
        {label}
      </div>
    </div>
  );
}

function AddPageButton({
  onCreate,
}: {
  onCreate: (page: CustomPage) => void;
}) {
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const { t } = useTranslation('navigation');
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const addCustomPage = useSettingsStore((s) => s.addCustomPage);

  const handleConfirm = useCallback(() => {
    const trimmed = name.trim();
    if (trimmed && currentAccount) {
      addCustomPage(currentAccount.id, trimmed).then((page) => {
        onCreate(page);
      });
    }
    setIsCreating(false);
    setName('');
  }, [name, currentAccount, addCustomPage, onCreate]);

  const handleCancel = useCallback(() => {
    setIsCreating(false);
    setName('');
  }, []);

  // Close popover on outside click
  useEffect(() => {
    if (!isCreating) return;
    const handler = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(e.target as Node)
      ) {
        handleCancel();
      }
    };
    // Small delay to avoid conflicting with the button click
    setTimeout(() => document.addEventListener('mousedown', handler), 0);
    return () => document.removeEventListener('mousedown', handler);
  }, [isCreating, handleCancel]);

  return (
    <div className={styles.addPageRow}>
      {/* + button */}
      <button
        ref={buttonRef}
        className={styles.addPageButton}
        onClick={() => {
          setIsCreating(true);
          setTimeout(() => inputRef.current?.focus(), 100);
        }}
        aria-label={t('add_page')}
        title={t('add_page')}
      >
        <Plus size={20} />
      </button>

      {/* Popover create row — rendered outside sidebar flow */}
      {isCreating && (
        <div
          ref={popoverRef}
          style={{
            position: 'fixed',
            left: 56, /* 48px sidebar + 8px gap */
            top: buttonRef.current
              ? buttonRef.current.getBoundingClientRect().top + 48 + 4
              : '50%',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '6px 10px',
            background: 'var(--bg-elevated)',
            borderRadius: 8,
            boxShadow: 'var(--shadow-lg)',
            zIndex: 300,
            border: '1px solid var(--border-subtle)',
          }}
        >
          <FileText size={20} style={{ color: 'var(--accent-primary)', flexShrink: 0 }} />
          <input
            ref={inputRef}
            value={name}
            onChange={(e) => setName(e.target.value.slice(0, 20))}
            onBlur={(e) => {
              // Only confirm if the blur is not caused by clicking inside the popover
              if (popoverRef.current && !popoverRef.current.contains(e.relatedTarget as Node)) {
                handleConfirm();
              }
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleConfirm();
              if (e.key === 'Escape') handleCancel();
            }}
            placeholder={t('add_page_placeholder')}
            maxLength={20}
            autoFocus
            aria-label={t('add_page_placeholder')}
            style={{
              padding: '6px 10px',
              fontSize: 14,
              border: '1px solid var(--accent-primary)',
              borderRadius: 6,
              background: 'transparent',
              color: 'var(--text-primary)',
              fontFamily: 'inherit',
              outline: 'none',
              width: 160,
            }}
          />
        </div>
      )}
    </div>
  );
}

export function SideNavigation() {
  const navigate = useNavigate();
  const location = useLocation();
  const vaultLock = useVaultStore((s) => s.lock);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const { t } = useTranslation('navigation');

  // Bind lock action
  const items = secondaryItems.map((item) =>
    item.type === 'action' ? { ...item, action: vaultLock } as NavAction : item
  );

  const isWorkspaceSectionActive = (sectionPath: string): boolean => {
    // Custom pages are at /workspace/custom/:id — they never match section-based routes
    if (location.pathname.startsWith('/workspace/custom/')) return false;
    if (!location.pathname.startsWith('/workspace')) return false;
    const section = sectionPath.split('section=')[1];
    if (!section) return !location.search.includes('section=');
    return location.search.includes(`section=${section}`);
  };

  const isCustomPageActive = (pageId: string): boolean => {
    return location.pathname === `/workspace/custom/${pageId}`;
  };

  const handleCustomPageNavigate = (page: CustomPage) => {
    navigate(`/workspace/custom/${page.id}`);
  };

  return (
    <nav className={styles.sideNav} aria-label={t('home')}>
      <div className={styles.logo}>S</div>

      <div className={styles.navPrimary}>
        {/* Default pages */}
        {primaryItems.map((item) => {
          const isActive =
            item.path === '/'
              ? location.pathname === '/'
              : isWorkspaceSectionActive(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={item.icon}
              label={t(item.labelKey)}
              isActive={isActive}
              onClick={() => navigate(item.path)}
            />
          );
        })}

        {/* Custom pages — dynamic list */}
        {customPages.map((page) => (
          <NavButton
            key={page.id}
            path={`/workspace/custom/${page.id}`}
            Icon={FileText}
            label={page.name}
            isActive={isCustomPageActive(page.id)}
            onClick={() => handleCustomPageNavigate(page)}
          />
        ))}

        {/* Add page button */}
        <AddPageButton onCreate={(page) => {
          navigate(`/workspace/custom/${page.id}`);
        }} />
      </div>

      <div className={styles.navSecondary}>
        {items.map((item, i) => {
          if (item.type === 'action') {
            return (
              <NavButton
                key={`action-${i}`}
                Icon={item.icon}
                label={t(item.labelKey)}
                onClick={item.action}
              />
            );
          }
          const isActive = item.path === '/'
            ? location.pathname === '/'
            : location.pathname.startsWith(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={item.icon}
              label={t(item.labelKey)}
              isActive={isActive}
              onClick={() => navigate(item.path)}
            />
          );
        })}
      </div>
    </nav>
  );
}
