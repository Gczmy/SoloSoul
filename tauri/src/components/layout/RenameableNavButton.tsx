import React, { useState, useRef, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type { LucideIcon } from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { CustomPage } from '@/stores/settingsStore';
import { NavButton } from './NavButton';
import {
  CUSTOM_ICON_MAP,
  resolveCustomIcon,
  ICON_CATEGORIES,
  CATEGORY_LABELS,
  type CustomIconId,
} from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS } from './useNavigationItems';
import { ICON_SIZE } from '@/lib/constants';

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

  // Compute max height for icon grid based on available viewport space
  const scrollMaxHeight = useMemo(() => {
    if (!renameCardRect) return 280;
    const nonInputHeight = 72;
    if (isBottom) {
      return Math.max(120, Math.min(280, renameCardRect.top - 80));
    }
    const topEdge = isHorizontal ? renameCardRect.bottom + 8 : renameCardRect.top;
    const available = window.innerHeight - topEdge - 16 - nonInputHeight;
    return Math.max(120, Math.min(280, available));
  }, [renameCardRect, isHorizontal, isBottom]);

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
      0,
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
      {createPortal(
        <AnimatePresence>
          {isRenaming && (
            <motion.div
              ref={popoverRef}
              initial={{ opacity: 0, y: -6, scale: 0.96 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -6, scale: 0.96 }}
              transition={{ duration: 0.15, ease: 'easeOut' }}
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
                transformOrigin: 'top',
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
                      fontSize: 'var(--text-body)',
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
                      <span
                        style={{
                          fontSize: 'var(--text-badge)',
                          color: '#e74c3c',
                          whiteSpace: 'nowrap',
                        }}
                      >
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
                          fontSize: 'var(--text-badge)',
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

              {/* Icon picker grid — category sections (scrollable) */}
              {showIconPicker && (
                <div
                  style={{
                    maxHeight: scrollMaxHeight,
                    overflowY: 'auto',
                    overflowX: 'hidden',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 8,
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
                              onClick={() => {
                                setSelectedIconId(id);
                                setShowIconPicker(false);
                              }}
                              onMouseEnter={(e) => {
                                if (id !== selectedIconId) {
                                  e.currentTarget.style.background =
                                    'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                                }
                              }}
                              onMouseLeave={(e) => {
                                if (id !== selectedIconId) {
                                  e.currentTarget.style.background = 'transparent';
                                  e.currentTarget.style.borderColor = 'transparent';
                                }
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
                                    ? '2px solid var(--accent-primary)'
                                    : '1px solid transparent',
                                background:
                                  selectedIconId === id ? 'rgba(91,124,153,0.08)' : 'transparent',
                                cursor: 'pointer',
                                transition: 'all 0.1s ease',
                              }}
                            >
                              <IconComp
                                size={ICON_SIZE.lg}
                                style={{
                                  color:
                                    selectedIconId === id
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
              )}
            </motion.div>
          )}
        </AnimatePresence>,
        document.body,
      )}
    </div>
  );
}
