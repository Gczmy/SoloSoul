import { useState, useRef, useCallback, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useLocation, useNavigate } from 'react-router-dom';
import { Plus } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import styles from './SideNavigation.module.css';
import { useVaultStore } from '@/stores/vaultStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { useTranslation } from 'react-i18next';
import type { CustomPage } from '@/stores/settingsStore';
import {
  PAGE_ICON_MAP,
  CUSTOM_ICON_MAP,
  resolveCustomIcon,
  DEFAULT_CUSTOM_ICON,
  type PageIconKey,
  type CustomIconId,
} from '@/lib/pageIcons';

// =============================================================================
// Nav item type definitions (icons sourced from PAGE_ICON_MAP — §7.4 SSOT)
// =============================================================================

interface NavLink {
  type: 'link';
  path: string;
  iconKey: PageIconKey;
  labelKey: string;
}

interface NavAction {
  type: 'action';
  iconKey: PageIconKey;
  labelKey: string;
  action: () => void;
}

type NavItem = NavLink | NavAction;

const primaryItems: NavLink[] = [
  { type: 'link', path: '/', iconKey: 'home', labelKey: 'home' },
  { type: 'link', path: '/workspace?section=identity', iconKey: 'profile', labelKey: 'profile' },
  { type: 'link', path: '/workspace?section=travel', iconKey: 'travel', labelKey: 'travel' },
  { type: 'link', path: '/workspace?section=financial', iconKey: 'financial', labelKey: 'financial' },
  { type: 'link', path: '/workspace?section=professional', iconKey: 'professional', labelKey: 'professional' },
];

const secondaryItems: NavItem[] = [
  { type: 'action', iconKey: 'lock', labelKey: 'lock_vault', action: () => {} },
  { type: 'link', path: '/search', iconKey: 'search', labelKey: 'search' },
  { type: 'link', path: '/plugins', iconKey: 'plugins', labelKey: 'plugin' },
  { type: 'link', path: '/llm-chat', iconKey: 'ai_chat', labelKey: 'ai_chat' },
  { type: 'link', path: '/settings', iconKey: 'settings', labelKey: 'settings' },
];

// =============================================================================
// NavButton — renders a single sidebar button with portal-based name card
// =============================================================================

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
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [cardPos, setCardPos] = useState<{ top: number; left: number } | null>(null);
  const [isHovered, setIsHovered] = useState(false);

  const updatePosition = useCallback(() => {
    if (wrapperRef.current) {
      const rect = wrapperRef.current.getBoundingClientRect();
      setCardPos({
        top: rect.top + rect.height / 2,
        left: rect.right + 8,
      });
    }
  }, []);

  const handleMouseEnter = useCallback(() => {
    setIsHovered(true);
    updatePosition();
  }, [updatePosition]);

  const handleMouseLeave = useCallback(() => {
    setIsHovered(false);
  }, []);

  // Update position on scroll/resize while hovered
  useEffect(() => {
    if (!isHovered) return;
    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);
    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [isHovered, updatePosition]);

  const nameCard = isHovered ? (
    <div
      className={styles.nameCardPortal}
      style={{
        position: 'fixed',
        top: cardPos?.top ?? 0,
        left: cardPos?.left ?? 0,
        transform: 'translateY(-50%)',
        zIndex: 200,
      }}
      role="tooltip"
      aria-hidden="true"
    >
      {label}
    </div>
  ) : null;

  return (
    <div
      ref={wrapperRef}
      className={styles.navItemWrapper}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
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
      {createPortal(nameCard, document.body)}
    </div>
  );
}

// =============================================================================
// AddPageButton — "+" button with popover for name + icon selection
// =============================================================================

function AddPageButton({
  onCreate,
}: {
  onCreate: (page: CustomPage) => void;
}) {
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState('');
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(DEFAULT_CUSTOM_ICON);
  const [showIconPicker, setShowIconPicker] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const { t } = useTranslation('navigation');
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const addCustomPage = useSettingsStore((s) => s.addCustomPage);

  const handleConfirm = useCallback(() => {
    const trimmed = name.trim();
    if (trimmed && currentAccount) {
      addCustomPage(currentAccount.id, trimmed, selectedIconId).then((page) => {
        onCreate(page);
      });
    }
    setIsCreating(false);
    setName('');
    setSelectedIconId(DEFAULT_CUSTOM_ICON);
    setShowIconPicker(false);
  }, [name, selectedIconId, currentAccount, addCustomPage, onCreate]);

  const handleCancel = useCallback(() => {
    setIsCreating(false);
    setName('');
    setSelectedIconId(DEFAULT_CUSTOM_ICON);
    setShowIconPicker(false);
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
        // If input has text → create page; if empty → cancel
        handleConfirm();
      }
    };
    // Small delay to avoid conflicting with the button click
    setTimeout(() => document.addEventListener('mousedown', handler), 0);
    return () => document.removeEventListener('mousedown', handler);
  }, [isCreating, handleConfirm]);

  const SelectedIcon = CUSTOM_ICON_MAP[selectedIconId];

  // Hover name card (same portal pattern as NavButton)
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [cardPos, setCardPos] = useState<{ top: number; left: number } | null>(null);
  const [isHovered, setIsHovered] = useState(false);

  const updateCardPosition = useCallback(() => {
    if (wrapperRef.current) {
      const rect = wrapperRef.current.getBoundingClientRect();
      setCardPos({ top: rect.top + rect.height / 2, left: rect.right + 8 });
    }
  }, []);

  const handleMouseEnter = useCallback(() => {
    setIsHovered(true);
    updateCardPosition();
  }, [updateCardPosition]);

  const handleMouseLeave = useCallback(() => {
    setIsHovered(false);
  }, []);

  useEffect(() => {
    if (!isHovered) return;
    window.addEventListener('scroll', updateCardPosition, true);
    window.addEventListener('resize', updateCardPosition);
    return () => {
      window.removeEventListener('scroll', updateCardPosition, true);
      window.removeEventListener('resize', updateCardPosition);
    };
  }, [isHovered, updateCardPosition]);

  const nameCard = isHovered && !isCreating ? (
    <div
      className={styles.nameCardPortal}
      style={{
        position: 'fixed',
        top: cardPos?.top ?? 0,
        left: cardPos?.left ?? 0,
        transform: 'translateY(-50%)',
        zIndex: 200,
      }}
      role="tooltip"
      aria-hidden="true"
    >
      {t('add_page')}
    </div>
  ) : null;

  return (
    <div className={styles.addPageRow}>
      {/* + button */}
      <div
        ref={wrapperRef}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        <button
          ref={buttonRef}
          className={styles.addPageButton}
          onClick={() => {
            setIsCreating(true);
            setSelectedIconId(DEFAULT_CUSTOM_ICON);
            setShowIconPicker(false);
            setTimeout(() => inputRef.current?.focus(), 100);
          }}
          aria-label={t('add_page')}
        >
          <Plus size={20} />
        </button>
        {createPortal(nameCard, document.body)}
      </div>

      {/* Popover create row — rendered outside sidebar flow */}
      {isCreating && (
        <div
          ref={popoverRef}
          style={{
            position: 'fixed',
            left: buttonRef.current
              ? buttonRef.current.getBoundingClientRect().right + 8
              : 56,
            top: buttonRef.current
              ? buttonRef.current.getBoundingClientRect().top
              : '50%',
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            padding: '10px 12px',
            background: 'var(--bg-elevated)',
            borderRadius: 8,
            boxShadow: 'var(--shadow-lg)',
            zIndex: 300,
            border: '1px solid var(--border-subtle)',
          }}
        >
          {/* Name input row */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            {/* Icon picker trigger */}
            <button
              onClick={() => setShowIconPicker(!showIconPicker)}
              style={{
                width: 32,
                height: 32,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                cursor: 'pointer',
                flexShrink: 0,
              }}
              title={t('add_page_placeholder') ?? 'Choose icon'}
              aria-label={t("navigation:add_page")}
            >
              <SelectedIcon size={18} style={{ color: 'var(--accent-primary)' }} />
            </button>
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

          {/* Icon picker grid */}
          {showIconPicker && (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(5, 1fr)',
                gap: 4,
                padding: '4px 0',
              }}
            >
              {(Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]).map(([id, IconComp]) => (
                <button
                  key={id}
                  onClick={() => {
                    setSelectedIconId(id);
                    setShowIconPicker(false);
                  }}
                  style={{
                    width: 32,
                    height: 32,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    borderRadius: 6,
                    border: id === selectedIconId
                      ? '2px solid var(--accent-primary)'
                      : '1px solid transparent',
                    background: id === selectedIconId
                      ? 'var(--accent-primary-transparent, rgba(91,124,153,0.1))'
                      : 'transparent',
                    cursor: 'pointer',
                  }}
                  title={id}
                  aria-label={id}
                >
                  <IconComp
                    size={16}
                    style={{
                      color: id === selectedIconId
                        ? 'var(--accent-primary)'
                        : 'var(--text-secondary)',
                    }}
                  />
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// SideNavigation — main sidebar component
// =============================================================================

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
        {/* Default pages — icons from PAGE_ICON_MAP (§7.4 SSOT) */}
        {primaryItems.map((item) => {
          const isActive =
            item.path === '/'
              ? location.pathname === '/'
              : isWorkspaceSectionActive(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={PAGE_ICON_MAP[item.iconKey]}
              label={t(item.labelKey)}
              isActive={isActive}
              onClick={() => navigate(item.path)}
            />
          );
        })}

        {/* Custom pages — icons from CUSTOM_ICON_MAP via iconId (§9.8) */}
        {customPages.map((page) => (
          <NavButton
            key={page.id}
            path={`/workspace/custom/${page.id}`}
            Icon={resolveCustomIcon(page.iconId)}
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
                Icon={PAGE_ICON_MAP[item.iconKey]}
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
              Icon={PAGE_ICON_MAP[item.iconKey]}
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
