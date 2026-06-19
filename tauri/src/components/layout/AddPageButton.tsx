import React, { useState, useRef, useCallback, useEffect } from 'react';
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
  type CustomIconId,
} from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS } from './useNavigationItems';

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
      </div>      {/* Popover create row — portaled to body so it sits above sidebar/tooltips */}
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
                transformOrigin: 'top',
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
            </motion.div>
          )}
        </AnimatePresence>,
        document.body,
      )}
    </div>
  );
}

