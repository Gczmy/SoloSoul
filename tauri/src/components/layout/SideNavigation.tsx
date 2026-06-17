import React, { useState, useRef, useCallback, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useLocation, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { Plus } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import styles from './SideNavigation.module.css';
import { useSettingsStore } from '@/stores/settingsStore';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { useAuthStore } from '@/stores/authStore';
import { useTranslation } from 'react-i18next';
import type { CustomPage } from '@/stores/settingsStore';
import { SearchPopover } from './SearchPopover';
import { NavButton } from './NavButton';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { AiQuickChatPopover } from './AiQuickChatPopover';
import { OcrQuickScanPopover } from './OcrQuickScanPopover';import {
  useActiveCustomPages,
  useBoundNavActions,
  useAiQuickChat,
  useOcrQuickScan,
  SYSTEM_PAGE_KEYS,
  primaryItems,
} from './useNavigationItems';
import {
  PAGE_ICON_MAP,
  CUSTOM_ICON_MAP,
  resolveCustomIcon,
  DEFAULT_CUSTOM_ICON,
  type CustomIconId,
} from '@/lib/pageIcons';


// =============================================================================
// RenameableNavButton — custom page button with double-click rename
// =============================================================================

export function RenameableNavButton({
  page,
  isActive,
  onClick,
  position = 'left',
}: {
  page: CustomPage;
  isActive: boolean;
  onClick: () => void;
  position?: import('./NavButton').NavPosition;
}) {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const isRight = position === 'right';
  const { t } = useTranslation(['navigation', 'common']);
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState(page.name);
  const [renameError, setRenameError] = useState(false);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(page.iconId as CustomIconId);
  const [showIconPicker, setShowIconPicker] = useState(false);
  const [renameCardRect, setRenameCardRect] = useState<DOMRect | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleDoubleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setRenameValue(page.name);
    setSelectedIconId(page.iconId as CustomIconId);
    setShowIconPicker(false);
    setRenameCardRect(wrapperRef.current?.getBoundingClientRect() || null);
    setIsRenaming(true);
    setTimeout(() => inputRef.current?.focus(), 50);
  };

  const handleConfirmRename = async () => {
    const trimmed = renameValue.trim();
    if (!trimmed) {
      setIsRenaming(false);
      return;
    }
    const nameChanged = trimmed !== page.name;
    const iconChanged = selectedIconId !== page.iconId;
    if (!nameChanged && !iconChanged) {
      setIsRenaming(false);
      return;
    }
    // Check for duplicate page names (only if name changed)
    if (nameChanged) {
      const store = useSettingsStore.getState();
      const existingNames = [
        ...SYSTEM_PAGE_KEYS.map((k) => t(k)),
        ...store.settings.customPages
          .filter((p) => p.id !== page.id && !p.deletedAt)
          .map((p) => p.name),
      ];
      if (existingNames.some((n) => n.toLowerCase() === trimmed.toLowerCase())) {
        setRenameError(true);
        return;
      }
    }
    // Update the object in the objects table
    try {
      await invoke('object_update', {
        objectId: page.id,
        input: { name: trimmed, properties: {}, iconName: selectedIconId },
      });
    } catch {
      // F020: stop and leave the previous name in place if persistence failed
      setRenameError(true);
      return;
    }
    // Update Zustand state so sidebar reflects the change
    const store = useSettingsStore.getState();
    store.updateSetting(
      accountId || '',
      'customPages',
      store.settings.customPages.map((p) =>
        p.id === page.id ? { ...p, name: trimmed, iconId: selectedIconId } : p,
      ),
    );
    setIsRenaming(false);
  };

  const handleCancelRename = () => {
    setIsRenaming(false);
    setRenameValue(page.name);
    setSelectedIconId(page.iconId as CustomIconId);
    setShowIconPicker(false);
  };

  // Use ref to always call the latest handleConfirmRename (avoids stale closure)
  const handleConfirmRenameRef = useRef(handleConfirmRename);
  handleConfirmRenameRef.current = handleConfirmRename;

  // Close on outside click (exclude the portaled rename card itself)
  useEffect(() => {
    if (!isRenaming) return;
    const handler = (e: MouseEvent) => {
      if (
        inputRef.current &&
        !inputRef.current.contains(e.target as Node) &&
        wrapperRef.current &&
        !wrapperRef.current.contains(e.target as Node) &&
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node)
      ) {
        handleConfirmRenameRef.current();
      }
    };
    outsideClickTimeoutRef.current = setTimeout(
      () => document.addEventListener('mousedown', handler),
      0
    );
    return () => {
      if (outsideClickTimeoutRef.current) {
        clearTimeout(outsideClickTimeoutRef.current);
      }
      document.removeEventListener('mousedown', handler);
    };
  }, [isRenaming]);

  return (
    <div ref={wrapperRef} style={{ position: 'relative' }} onDoubleClick={handleDoubleClick}>
      <NavButton
        path={`/workspace/custom/${page.id}`}
        Icon={resolveCustomIcon(page.iconId)}
        label={page.name}
        isActive={isActive}
        onClick={onClick}
        position={position}
      />
      {isRenaming &&
        createPortal(
          <div
            ref={popoverRef}
            style={{
              position: 'fixed',
              left: isHorizontal
                ? renameCardRect
                  ? renameCardRect.left
                  : 56
                : isRight
                  ? 'auto'
                  : renameCardRect
                    ? renameCardRect.right + 8
                    : 56,
              right: isRight
                ? renameCardRect
                  ? window.innerWidth - renameCardRect.left + 8
                  : 56
                : 'auto',
              top: isBottom
                ? 'auto'
                : renameCardRect
                  ? isHorizontal
                    ? renameCardRect.bottom + 8
                    : renameCardRect.top
                  : '50%',
              bottom: isBottom
                ? renameCardRect
                  ? window.innerHeight - renameCardRect.top + 8
                  : 56
                : 'auto',
              display: 'flex',
              flexDirection: 'column',
              gap: 8,
              padding: '6px 10px',
              background: 'var(--bg-elevated)',
              borderRadius: 8,
              boxShadow: 'var(--shadow-lg)',
              zIndex: 300,
              border: '1px solid var(--border-subtle)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'flex-start', gap: 6 }}>
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
                title={t('navigation:add_page_placeholder') ?? 'Choose icon'}
              >
                {React.createElement(CUSTOM_ICON_MAP[selectedIconId], {
                  size: 18,
                  style: { color: 'var(--accent-primary)' },
                })}
              </button>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                <input
                  ref={inputRef}
                  value={renameValue}
                  onChange={(e) => {
                    setRenameValue(e.target.value.slice(0, 30));
                    setRenameError(false);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleConfirmRename();
                    if (e.key === 'Escape') handleCancelRename();
                  }}
                  maxLength={30}
                  autoFocus
                  style={{
                    padding: '6px 10px',
                    fontSize: 14,
                    border: renameError ? '1px solid #e74c3c' : '1px solid var(--accent-primary)',
                    borderRadius: 6,
                    background: 'transparent',
                    color: 'var(--text-primary)',
                    fontFamily: 'inherit',
                    outline: 'none',
                    width: 140,
                    animation: renameError ? 'shake 0.4s ease' : 'none',
                  }}
                />
                {renameError && (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      gap: 8,
                    }}
                  >
                    <span style={{ fontSize: 11, color: '#e74c3c', whiteSpace: 'nowrap' }}>
                      {t('page_name_exists')}
                    </span>
                    <button
                      onClick={handleCancelRename}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.color = 'var(--accent-primary)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.color = 'var(--text-tertiary)';
                      }}
                      style={{
                        fontSize: 11,
                        color: 'var(--text-tertiary)',
                        background: 'none',
                        border: 'none',
                        cursor: 'pointer',
                        padding: 0,
                        transition: 'color 0.15s ease',
                      }}
                    >
                      {t('common:cancel')}
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Icon picker grid */}
            {showIconPicker && (
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: 'repeat(6, 1fr)',
                  gap: 4,
                  padding: '4px 0',
                }}
              >
                {(Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]).map(
                  ([id, IconComp]) => (
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
                        border:
                          selectedIconId === id
                            ? '1px solid var(--accent-primary)'
                            : '1px solid transparent',
                        background:
                          selectedIconId === id ? 'rgba(91,124,153,0.08)' : 'transparent',
                        cursor: 'pointer',
                      }}
                    >
                      <IconComp
                        size={18}
                        style={{
                          color:
                            selectedIconId === id
                              ? 'var(--accent-primary)'
                              : 'var(--text-secondary)',
                        }}
                      />
                    </button>
                  ),
                )}
              </div>
            )}
          </div>,
          document.body,
        )}
    </div>
  );
}

// =============================================================================
// AddPageButton — "+" button with popover for name + icon selection
// =============================================================================

export function AddPageButton({
  onCreate,
  position = 'left',
}: {
  onCreate: (page: CustomPage) => void;
  position?: import('./NavButton').NavPosition;
}) {
  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const isRight = position === 'right';
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState('');
  const [nameError, setNameError] = useState(false);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(DEFAULT_CUSTOM_ICON);
  const [buttonRect, setButtonRect] = useState<DOMRect | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { t } = useTranslation(['navigation', 'common']);
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const addCustomPage = useSettingsStore((s) => s.addCustomPage);

  const handleCancel = useCallback(() => {
    setIsCreating(false);
    setName('');
    setNameError(false);
    setSelectedIconId(DEFAULT_CUSTOM_ICON);
  }, []);

  const handleConfirm = useCallback(() => {
    const trimmed = name.trim();
    if (!trimmed || !currentAccount) {
      handleCancel();
      return;
    }
    // Check for duplicate page names
    const store = useSettingsStore.getState();
    const existingNames = [
      ...SYSTEM_PAGE_KEYS.map((k) => t(k)),
      ...store.settings.customPages.filter((p) => !p.deletedAt).map((p) => p.name),
    ];
    if (existingNames.some((n) => n.toLowerCase() === trimmed.toLowerCase())) {
      setNameError(true);
      return;
    }
    addCustomPage(currentAccount.id, trimmed, selectedIconId).then((page) => {
      onCreate(page);
    });
    handleCancel();
  }, [name, selectedIconId, currentAccount, addCustomPage, onCreate, t, handleCancel]);

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
    outsideClickTimeoutRef.current = setTimeout(
      () => document.addEventListener('mousedown', handler),
      0
    );
    return () => {
      if (outsideClickTimeoutRef.current) {
        clearTimeout(outsideClickTimeoutRef.current);
      }
      document.removeEventListener('mousedown', handler);
    };
  }, [isCreating, handleConfirm]);

  // Hover name card (same portal pattern as NavButton)
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [cardStyle, setCardStyle] = useState<React.CSSProperties | null>(null);
  const [isHovered, setIsHovered] = useState(false);

  const updateCardPosition = useCallback(() => {
    if (wrapperRef.current) {
      const rect = wrapperRef.current.getBoundingClientRect();
      if (isHorizontal) {
        if (isBottom) {
          setCardStyle({
            top: 'auto',
            bottom: window.innerHeight - rect.top + 8,
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        } else {
          setCardStyle({
            top: rect.bottom + 8,
            bottom: 'auto',
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        }
      } else if (isRight) {
        setCardStyle({
          top: rect.top + rect.height / 2,
          bottom: 'auto',
          left: 'auto',
          right: window.innerWidth - rect.left + 8,
          transform: 'translateY(-50%)',
        });
      } else {
        setCardStyle({
          top: rect.top + rect.height / 2,
          bottom: 'auto',
          left: rect.right + 8,
          transform: 'translateY(-50%)',
        });
      }
    }
  }, [isHorizontal, isBottom, isRight]);

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

  const nameCard =
    isHovered && !isCreating ? (
      <div
        className={isHorizontal ? styles.nameCardPortalHorizontal : styles.nameCardPortal}
        style={{
          position: 'fixed',
          ...cardStyle,
          zIndex: 200,
        }}
        role="tooltip"
        aria-hidden="true"
      >
        {t('add_page')}
      </div>
    ) : null;

  return (
    <div className={styles.addPageRow} style={isHorizontal ? { flexDirection: 'row' } : {}}>
      {/* + button */}
      <div
        ref={wrapperRef}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        style={isHorizontal ? { width: 40, height: 40 } : undefined}
      >
        <button
          ref={buttonRef}
          className={styles.addPageButton}
          style={isHorizontal ? { width: 40, height: 40, borderRadius: 10 } : {}}
          onClick={() => {
            setButtonRect(buttonRef.current?.getBoundingClientRect() || null);
            setIsCreating(true);
            setSelectedIconId(DEFAULT_CUSTOM_ICON);
            setTimeout(() => inputRef.current?.focus(), 100);
          }}
          aria-label={t('add_page')}
          data-tauri-drag-region="false"
        >
          <Plus size={20} />
        </button>
        {createPortal(nameCard, document.body)}
      </div>

      {/* Popover create row — portaled to body so it sits above sidebar/tooltips */}
      {isCreating &&
        createPortal(
          <div
            ref={popoverRef}
            style={{
              position: 'fixed',
              left: isHorizontal
                ? buttonRect
                  ? buttonRect.left
                  : 56
                : isRight
                  ? 'auto'
                  : buttonRect
                    ? buttonRect.right + 8
                    : 56,
              right: isRight
                ? buttonRect
                  ? window.innerWidth - buttonRect.left + 8
                  : 56
                : 'auto',
              top: isBottom
                ? 'auto'
                : buttonRect
                  ? isHorizontal
                    ? buttonRect.bottom + 8
                    : buttonRect.top
                  : '50%',
              bottom: isBottom
                ? buttonRect
                  ? window.innerHeight - buttonRect.top + 8
                  : 56
                : 'auto',
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
            {/* Name input */}
            <input
              ref={inputRef}
              value={name}
              onChange={(e) => {
                setName(e.target.value.slice(0, 20));
                setNameError(false);
              }}
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
                border: nameError ? '1px solid #e74c3c' : '1px solid var(--accent-primary)',
                borderRadius: 6,
                background: 'transparent',
                color: 'var(--text-primary)',
                fontFamily: 'inherit',
                outline: 'none',
                width: '100%',
                boxSizing: 'border-box',
                animation: nameError ? 'shake 0.4s ease' : 'none',
              }}
            />
            {nameError && (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 8,
                }}
              >
                <span style={{ fontSize: 11, color: '#e74c3c', whiteSpace: 'nowrap' }}>
                  {t('page_name_exists')}
                </span>
                <button
                  onClick={handleCancel}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.color = 'var(--text-tertiary)';
                  }}
                  style={{
                    fontSize: 11,
                    color: 'var(--text-tertiary)',
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    padding: 0,
                    transition: 'color 0.15s ease',
                  }}
                >
                  {t('common:cancel')}
                </button>
              </div>
            )}

            {/* Icon picker grid — always visible for quick selection */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{t('select_icon')}</span>
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: 'repeat(6, 1fr)',
                  gap: 4,
                }}
              >
                {(Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]).map(
                  ([id, IconComp]) => (
                    <button
                      key={id}
                      onMouseDown={(e) => e.preventDefault()} // prevent input blur so selectedIconId updates before confirm
                      onClick={() => setSelectedIconId(id)}
                      style={{
                        width: 32,
                        height: 32,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        borderRadius: 6,
                        border:
                          id === selectedIconId
                            ? '2px solid var(--accent-primary)'
                            : '1px solid transparent',
                        background:
                          id === selectedIconId
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
                          color:
                            id === selectedIconId
                              ? 'var(--accent-primary)'
                              : 'var(--text-secondary)',
                        }}
                      />
                    </button>
                  ),
                )}
              </div>
            </div>
          </div>,
          document.body,
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
  const activeCustomPages = useActiveCustomPages();
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('navigation');

  const { items, showSearch, setShowSearch } = useBoundNavActions();
  const ocrQuickScanPlacement: import('./useNavigationItems').OcrQuickScanPlacement =
    sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const { ocrButtonRef, quickScanPos } = useOcrQuickScan(560, ocrQuickScanPlacement);
  const aiQuickChatPlacement: import('./useNavigationItems').AiQuickChatPlacement =
    sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);

  const { showQuickChat, setShowQuickChat, aiButtonRef, quickChatPos } = useAiQuickChat(
    520,
    aiQuickChatPlacement,
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

  const horizontalNavRef = useRef<HTMLDivElement>(null);

  const handleHorizontalWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    const el = horizontalNavRef.current;
    if (!el) return;
    const delta = e.deltaY + e.deltaX;
    if (delta === 0) return;
    // No native scrollbar is shown; all wheel/trackpad input scrolls horizontally.
    e.preventDefault();
    el.scrollBy({ left: delta, behavior: 'auto' });
  }, []);

  // Drag-to-scroll for touch/mouse on empty areas of the horizontal bar.
  const [isDragging, setIsDragging] = useState(false);
  const dragStartX = useRef(0);
  const dragStartScroll = useRef(0);

  const handlePointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || e.target !== e.currentTarget) return;
    setIsDragging(true);
    dragStartX.current = e.clientX;
    dragStartScroll.current = horizontalNavRef.current?.scrollLeft ?? 0;
    (e.currentTarget as HTMLDivElement).setPointerCapture?.(e.pointerId);
  }, []);

  const handlePointerMove = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!isDragging || !horizontalNavRef.current) return;
      const dx = e.clientX - dragStartX.current;
      horizontalNavRef.current.scrollLeft = dragStartScroll.current - dx;
    },
    [isDragging],
  );

  const handlePointerUp = useCallback(() => setIsDragging(false), []);

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

  const zoneStyle: React.CSSProperties = isHorizontal
    ? { flexDirection: 'row', width: 'auto', height: '100%', overflow: 'visible' }
    : { flexDirection: 'column', width: '100%', height: 'auto', overflow: 'visible' };

  return (
    <nav className={styles.sideNav} aria-label={t('home')} style={navStyle}>
      <ShieldLogo
        size={32}
        style={isHorizontal ? { marginBottom: 0, marginRight: 12 } : { marginBottom: 16 }}
      />

      {isHorizontal ? (
        <>
          {/* ══ Horizontal mode (top/bottom sidebar) ══ */}

          {/* HOME — always visible (static left) */}
          <NavButton
            path={primaryItems[0].path}
            Icon={PAGE_ICON_MAP[primaryItems[0].iconKey]}
            label={t(primaryItems[0].labelKey)}
            isActive={location.pathname === '/'}
            onClick={() => navigate(primaryItems[0].path)}
            position={sidebarPosition}
          />

          {/* Scrollable zone: identity / travel / financial / professional + custom pages (excludes AddPageButton) */}
          <div className={styles.navPrimaryHorizontalWrapper}>
            <div
              ref={horizontalNavRef}
              onWheel={handleHorizontalWheel}
              onPointerDown={handlePointerDown}
              onPointerMove={handlePointerMove}
              onPointerUp={handlePointerUp}
              onPointerLeave={handlePointerUp}
              className={`${styles.navPrimary} ${styles.navPrimaryHorizontal}`}
            >
              {primaryItems.slice(1).map((item) => (
                <NavButton
                  key={item.path}
                  path={item.path}
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  isActive={isWorkspaceSectionActive(item.path)}
                  onClick={() => navigate(item.path)}
                  position={sidebarPosition}
                />
              ))}
              {activeCustomPages.map((page) => (
                <RenameableNavButton
                  key={page.id}
                  page={page}
                  isActive={isCustomPageActive(page.id)}
                  onClick={() => handleCustomPageNavigate(page)}
                  position={sidebarPosition}
                />
              ))}
            </div>
          </div>
        </>
      ) : (
        /* ══ Vertical mode (left/right sidebar) — Home fixed top, Add fixed bottom, middle scrolls ══ */
        <>
          {/* HOME — fixed at top, outside scrollable zone */}
          <NavButton
            path={primaryItems[0].path}
            Icon={PAGE_ICON_MAP[primaryItems[0].iconKey]}
            label={t(primaryItems[0].labelKey)}
            isActive={location.pathname === '/'}
            onClick={() => navigate(primaryItems[0].path)}
            position={sidebarPosition}
          />

          {/* Scrollable zone: other primary pages + custom pages */}
          <div
            className={styles.navPrimary}
            style={{ ...zoneStyle, flex: 1, overflowX: 'hidden', overflowY: 'auto' }}
          >
            {primaryItems.slice(1).map((item) => (
              <NavButton
                key={item.path}
                path={item.path}
                Icon={PAGE_ICON_MAP[item.iconKey]}
                label={t(item.labelKey)}
                isActive={isWorkspaceSectionActive(item.path)}
                onClick={() => navigate(item.path)}
                position={sidebarPosition}
              />
            ))}

            {activeCustomPages.map((page) => (
              <RenameableNavButton
                key={page.id}
                page={page}
                isActive={isCustomPageActive(page.id)}
                onClick={() => handleCustomPageNavigate(page)}
                position={sidebarPosition}
              />
            ))}
          </div>

          {/* ADD PAGE — fixed at bottom, outside scrollable zone */}
          <AddPageButton
            onCreate={(page) => {
              navigate(`/workspace/custom/${page.id}`);
            }}
            position={sidebarPosition}
          />
        </>
      )}

      <div
        className={styles.navSecondary}
        style={{ ...zoneStyle, flexShrink: 0, marginTop: isHorizontal ? 0 : 4 }}
      >
        {/* AddPageButton — in the secondary zone for horizontal mode only */}
        {isHorizontal && (
          <AddPageButton
            onCreate={(page) => {
              navigate(`/workspace/custom/${page.id}`);
            }}
            position={sidebarPosition}
          />
        )}
        {items.map((item, i) => {
          if (item.type === 'action') {
            if (item.iconKey === 'ocr') {
              return (
                <div ref={ocrButtonRef} key="ocr" data-ocr-button="true">
                  <NavButton
                    Icon={PAGE_ICON_MAP[item.iconKey]}
                    label={t(item.labelKey)}
                    isActive={isOcrCardOpen}
                    onClick={item.action}
                    position={sidebarPosition}
                  />
                  {isOcrCardOpen &&
                    createPortal(
                      <OcrQuickScanPopover
                        position={quickScanPos}
                        onClose={() => useOcrScanStore.getState().setCardOpen(false)}
                        placement={ocrQuickScanPlacement}
                      />,
                      document.body,
                    )}
                </div>
              );
            }
            const isSearch = item.iconKey === 'search';
            return (
              <div key={`action-${i}`} style={{ position: 'relative' }}>
                <NavButton
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  onClick={item.action}
                  position={sidebarPosition}
                />
                {isSearch &&
                  showSearch &&
                  createPortal(
                    <SearchPopover onClose={() => setShowSearch(false)} />,
                    document.body,
                  )}
              </div>
            );
          }
          if (item.path === '/llm-chat') {
            return (
              <div ref={aiButtonRef} key={item.path} data-ai-button="true">
                <NavButton
                  path={item.path}
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  isActive={showQuickChat || location.pathname.startsWith(item.path)}
                  onClick={() => {
                    if (location.pathname.startsWith('/llm-chat')) return;
                    setShowQuickChat((prev) => !prev);
                  }}
                  position={sidebarPosition}
                />
                {showQuickChat &&
                  createPortal(
                    <AiQuickChatPopover
                      position={quickChatPos}
                      onClose={() => setShowQuickChat(false)}
                      placement={aiQuickChatPlacement}
                    />,
                    document.body,
                  )}
              </div>
            );
          }
          const isActive =
            item.path === '/' ? location.pathname === '/' : location.pathname.startsWith(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={PAGE_ICON_MAP[item.iconKey]}
              label={t(item.labelKey)}
              isActive={isActive}
              onClick={() => navigate(item.path)}
              position={sidebarPosition}
            />
          );
        })}
      </div>
    </nav>
  );
}
