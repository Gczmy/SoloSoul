import React, { useState, useRef, useCallback, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { Plus } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { CustomPage } from '@/stores/settingsStore';
import styles from './SideNavigation.module.css';
import {
  CUSTOM_ICON_MAP,
  DEFAULT_CUSTOM_ICON,
  ICON_CATEGORIES,
  CATEGORY_LABELS,
  type CustomIconId,
} from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS } from './useNavigationItems';
import { ICON_SIZE } from '@/lib/constants';

// =============================================================================
// AddPageButton — "+" button with popover for name + icon selection
// =============================================================================

export function AddPageButton({
  onCreate,
  position = 'left',
  className,
  buttonClassName,
  showLabel,
  showDescription,
}: {
  onCreate: (page: CustomPage) => void;
  position?: import('./NavButton').NavPosition;
  className?: string;
  buttonClassName?: string;
  showLabel?: boolean;
  showDescription?: boolean;
}) {
  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const isRight = position === 'right';
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [nameError, setNameError] = useState(false);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(DEFAULT_CUSTOM_ICON);
  const [buttonRect, setButtonRect] = useState<DOMRect | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Compute max height for icon picker scroll area based on available viewport space
  const scrollMaxHeight = useMemo(() => {
    if (!buttonRect) return 280;
    // name input(~40) + optional description input(~40) + gap/padding(~24) + label(~14)
    const nonInputHeight = showDescription ? 118 : 72;
    if (isBottom) {
      // Opens upward from button bottom
      return Math.max(120, Math.min(280, buttonRect.top - 80));
    }
    // Opens downward: horizontal (below button) or side (aligned to top)
    const topEdge = isHorizontal ? buttonRect.bottom + 8 : buttonRect.top;
    const available = window.innerHeight - topEdge - 16 - nonInputHeight;
    return Math.max(120, Math.min(280, available));
  }, [buttonRect, isHorizontal, isBottom, showDescription]);

  // Compute popover left position for horizontal mode with right-edge overflow protection.
  // When the + button is near the right edge (function area collapsed), clamp left so
  // the entire popover stays within the viewport.
  const horizontalPopoverLeft = useMemo(() => {
    if (!buttonRect || !isCreating) return 56;
    const ESTIMATED_WIDTH = 276; // ~212px icon grid + 24px padding + 40px buffer
    const MARGIN = 12;
    const idealLeft = buttonRect.left;
    const rightEdge = idealLeft + ESTIMATED_WIDTH + MARGIN;
    if (rightEdge > window.innerWidth) {
      return Math.max(MARGIN, window.innerWidth - ESTIMATED_WIDTH - MARGIN);
    }
    return idealLeft;
  }, [buttonRect, isCreating]);

  const { t } = useTranslation(['navigation', 'common']);
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const addCustomPage = useSettingsStore((s) => s.addCustomPage);

  const handleCancel = useCallback(() => {
    setIsCreating(false);
    setName('');
    setDescription('');
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
    const trimmedDesc = description.trim();
    addCustomPage(currentAccount.id, trimmed, selectedIconId, trimmedDesc || undefined).then(
      (page) => {
        onCreate(page);
      },
    );
    handleCancel();
  }, [name, description, selectedIconId, currentAccount, addCustomPage, onCreate, t, handleCancel]);

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
      0,
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
    <div
      className={`${styles.addPageRow} ${className || ''}`}
      data-add-page-zone="true"
      style={className ? undefined : isHorizontal ? { flexDirection: 'row' } : {}}
    >
      {/* + button */}
      <div
        ref={wrapperRef}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        style={className || !isHorizontal ? undefined : { width: 40, height: 40 }}
      >
        <button
          ref={buttonRef}
          className={`${styles.addPageButton} ${buttonClassName || ''}`}
          style={
            buttonClassName
              ? undefined
              : isHorizontal
                ? { width: 40, height: 40, borderRadius: 10 }
                : {}
          }
          onClick={() => {
            setButtonRect(buttonRef.current?.getBoundingClientRect() || null);
            setIsCreating(true);
            setSelectedIconId(DEFAULT_CUSTOM_ICON);
            setTimeout(() => inputRef.current?.focus(), 100);
          }}
          aria-label={t('add_page')}
          data-tauri-drag-region="false"
        >
          <Plus size={ICON_SIZE.xl} />
          {showLabel && <span className={styles.addPageLabel}>{t('add_page')}</span>}
        </button>
        {createPortal(nameCard, document.body)}
      </div>{' '}
      {/* Popover create row — portaled to body so it sits above sidebar/tooltips */}
      {createPortal(
        <AnimatePresence>
          {isCreating && (
            <motion.div
              ref={popoverRef}
              initial={{ opacity: 0, y: -6, scale: 0.96 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -6, scale: 0.96 }}
              transition={{ duration: 0.15, ease: 'easeOut' }}
              style={{
                position: 'fixed',
                left: isBottom
                  ? 0
                  : isHorizontal
                    ? horizontalPopoverLeft
                    : isRight
                      ? 'auto'
                      : buttonRect
                        ? buttonRect.right + 8
                        : 56,
                right: isBottom
                  ? 0
                  : isRight
                    ? buttonRect
                      ? window.innerWidth - buttonRect.left + 8
                      : 56
                    : 'auto',
                margin: isBottom ? '0 auto' : undefined,
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
                transformOrigin: 'top',
                maxWidth: 'calc(100vw - 32px)',
                maxHeight:
                  'calc(100vh - env(safe-area-inset-top, 0px) - env(safe-area-inset-bottom, 0px) - 32px)',
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
                className={styles.addPageInput}
                data-error={nameError || undefined}
              />
              {showDescription && (
                <input
                  value={description}
                  onChange={(e) => setDescription(e.target.value.slice(0, 30))}
                  onBlur={(e) => {
                    if (
                      popoverRef.current &&
                      !popoverRef.current.contains(e.relatedTarget as Node)
                    ) {
                      handleConfirm();
                    }
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleConfirm();
                    if (e.key === 'Escape') handleCancel();
                  }}
                  placeholder={t('add_page_description_placeholder')}
                  maxLength={30}
                  aria-label={t('add_page_description_placeholder')}
                  className={styles.addPageInput}
                  data-secondary
                />
              )}
              {nameError && (
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: 8,
                  }}
                >
                  <span
                    style={{
                      fontSize: 'var(--text-badge)',
                      color: '#e74c3c',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {t('page_name_exists')}
                  </span>
                  <button onClick={handleCancel} className={styles.cancelTextBtn}>
                    {t('common:cancel')}
                  </button>
                </div>
              )}

              {/* Icon picker with category sections (scrollable) */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                  {t('select_icon')}
                </span>
                <div
                  style={{
                    maxHeight: scrollMaxHeight,
                    overflowY: 'auto',
                    overflowX: 'hidden',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 10,
                  }}
                >
                  {[
                    'general',
                    'security',
                    'identity',
                    'finance',
                    'travel',
                    'work',
                    'communication',
                    'health',
                    'education',
                    'life',
                    'nature',
                    'special',
                  ].map((cat) => {
                    const categoryIcons = (
                      Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]
                    ).filter(([id]) => ICON_CATEGORIES[id] === cat);
                    if (categoryIcons.length === 0) return null;
                    return (
                      <div key={cat}>
                        <div
                          style={{
                            fontSize: 'var(--text-badge)',
                            fontWeight: 500,
                            color: 'var(--text-tertiary)',
                            padding: '2px 0 4px',
                            borderBottom: '1px solid var(--border-subtle)',
                            marginBottom: 4,
                          }}
                        >
                          {t(`navigation:icon_category_${cat}`, CATEGORY_LABELS[cat])}
                        </div>
                        <div
                          style={{
                            display: 'grid',
                            gridTemplateColumns: 'repeat(6, 1fr)',
                            gap: 4,
                          }}
                        >
                          {categoryIcons.map(([id, IconComp]) => (
                            <button
                              key={id}
                              onMouseDown={(e) => e.preventDefault()}
                              onClick={() => setSelectedIconId(id)}
                              className={`${styles.iconPickerBtn} ${id === selectedIconId ? styles.iconPickerBtnSelected : ''}`}
                              title={id}
                              aria-label={id}
                            >
                              <IconComp
                                size={ICON_SIZE.md}
                                style={{
                                  color:
                                    id === selectedIconId
                                      ? 'var(--accent-primary)'
                                      : 'var(--text-secondary)',
                                }}
                              />
                            </button>
                          ))}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>

              {/* Cancel / Confirm buttons at bottom */}
              <div
                style={{
                  display: 'flex',
                  gap: 8,
                  justifyContent: 'flex-end',
                  paddingTop: 4,
                  borderTop: '1px solid var(--border-subtle)',
                }}
              >
                <button
                  onClick={handleCancel}
                  className={styles.cancelTextBtn}
                  style={{
                    padding: '6px 12px',
                    borderRadius: 6,
                    fontSize: 'var(--text-body-sm)',
                    background: 'transparent',
                    border: '1px solid var(--border-subtle)',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                  }}
                >
                  {t('common:cancel')}
                </button>
                <button
                  onClick={handleConfirm}
                  style={{
                    padding: '6px 12px',
                    borderRadius: 6,
                    fontSize: 'var(--text-body-sm)',
                    background: 'var(--accent-primary)',
                    border: 'none',
                    color: '#fff',
                    cursor: 'pointer',
                    fontWeight: 500,
                  }}
                >
                  {t('common:confirm')}
                </button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>,
        document.body,
      )}
    </div>
  );
}
