import React, { useState, useRef, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';
import { CUSTOM_ICON_MAP, type CustomIconId } from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS } from './useNavigationItems';
import { IconCategoryPicker } from './IconCategoryPicker';
import { SAFE_AREA_TOP, SAFE_AREA_BOTTOM } from '@/lib/constants';
import { isAndroidSync } from '@/lib/platform';
import { AndroidSheet } from '@/components/android/AndroidSheet';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import styles from './SideNavigation.module.css';

// =============================================================================
// CustomPageEditPopover — reusable editor for a custom page's icon/name/description
// =============================================================================

export interface CustomPageEditPopoverProps {
  page: CustomPage;
  isOpen: boolean;
  onClose: () => void;
  /** Rect of the trigger element, used to position the popover */
  triggerRect: DOMRect | null;
  /** Sidebar-like position; used for fine-tuned placement */
  position?: 'left' | 'right' | 'top' | 'bottom';
}

export function CustomPageEditPopover({
  page,
  isOpen,
  onClose,
  triggerRect,
  position = 'left',
}: CustomPageEditPopoverProps) {
  const isAndroid = isAndroidSync();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const isRight = position === 'right';
  const { t } = useTranslation(['navigation', 'common']);

  const [name, setName] = useState(page.name);
  const [description, setDescription] = useState(page.description || '');
  const [renameError, setRenameError] = useState(false);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(page.iconId as CustomIconId);
  const [showIconPicker, setShowIconPicker] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset state whenever the popover opens
  useEffect(() => {
    if (!isOpen) return;
    setName(page.name);
    setDescription(page.description || '');
    setSelectedIconId(page.iconId as CustomIconId);
    setRenameError(false);
    setShowIconPicker(false);
    // Android 由 Sheet 管理焦点，打开编辑面板时不自动唤起键盘。
    if (isAndroid) return;
    const timeout = setTimeout(() => inputRef.current?.focus(), 50);
    return () => clearTimeout(timeout);
  }, [isOpen, page, isAndroid]);

  // Compute max height for icon grid based on available viewport space
  const scrollMaxHeight = useMemo(() => {
    if (!triggerRect) return 280;
    const nonInputHeight = 72;
    if (isBottom) {
      return Math.max(120, Math.min(280, triggerRect.top - 80));
    }
    const topEdge = isHorizontal ? triggerRect.bottom + 8 : triggerRect.top;
    const available = window.innerHeight - topEdge - 16 - nonInputHeight;
    return Math.max(120, Math.min(280, available));
  }, [triggerRect, isHorizontal, isBottom]);

  const handleConfirm = async () => {
    const trimmed = name.trim();
    if (!trimmed) {
      onClose();
      return;
    }
    const trimmedDesc = description.trim();
    const nameChanged = trimmed !== page.name;
    const iconChanged = selectedIconId !== page.iconId;
    const descChanged = trimmedDesc !== (page.description || '');

    if (!nameChanged && !iconChanged && !descChanged) {
      onClose();
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
        input: {
          name: trimmed,
          properties: descChanged ? { description: trimmedDesc || undefined } : {},
          iconName: selectedIconId,
        },
      });
    } catch {
      setRenameError(true);
      return;
    }

    // Update Zustand state so sidebar and home cards reflect the change
    const store = useSettingsStore.getState();
    store.updateSetting(
      accountId || '',
      'customPages',
      store.settings.customPages.map((p) =>
        p.id === page.id
          ? { ...p, name: trimmed, iconId: selectedIconId, description: trimmedDesc || undefined }
          : p,
      ),
    );
    onClose();
  };

  const handleCancel = () => {
    setName(page.name);
    setDescription(page.description || '');
    setSelectedIconId(page.iconId as CustomIconId);
    setRenameError(false);
    setShowIconPicker(false);
    onClose();
  };

  // Use ref to always call the latest handleConfirm (avoids stale closure)
  const handleConfirmRef = useRef(handleConfirm);
  handleConfirmRef.current = handleConfirm;

  // Close on outside click
  useEffect(() => {
    if (!isOpen || isAndroid) return;
    const handler = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        handleConfirmRef.current();
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
  }, [isOpen, isAndroid]);

  // 触屏使用受视口和键盘安全区约束的底部面板，避免桌面锚点定位越过屏幕边缘。
  if (isAndroid) {
    return isOpen ? (
      <AndroidSheet
        title={t('common:material.edit_page', { name: page.name })}
        onClose={handleCancel}
      >
        <form
          className="android-page-form"
          onSubmit={(event) => {
            event.preventDefault();
            void handleConfirm();
          }}
        >
          <Input
            aria-label={t('add_page_placeholder')}
            placeholder={t('add_page_placeholder')}
            maxLength={30}
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              setRenameError(false);
            }}
            error={renameError ? t('page_name_exists') : undefined}
          />
          <Input
            aria-label={t('add_page_description_placeholder')}
            placeholder={t('add_page_description_placeholder')}
            maxLength={30}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
          <p>{t('select_icon')}</p>
          <fieldset className="android-icon-picker" aria-label={t('select_icon')}>
            <IconCategoryPicker selectedIconId={selectedIconId} onSelect={setSelectedIconId} />
          </fieldset>
          <div className="android-page-form-actions">
            <Button type="button" variant="secondary" onClick={handleCancel}>
              {t('common:cancel')}
            </Button>
            <Button type="submit" disabled={!name.trim()}>
              {t('common:save')}
            </Button>
          </div>
        </form>
      </AndroidSheet>
    ) : null;
  }

  return createPortal(
    isOpen && (
      <div
        ref={popoverRef}
        data-macos-glass="panel"
        className={styles.addPagePopover}
        style={{
          position: 'fixed',
          left: isBottom
            ? triggerRect
              ? triggerRect.left
              : 56
            : isHorizontal
              ? triggerRect
                ? triggerRect.left
                : 56
              : isRight
                ? 'auto'
                : triggerRect
                  ? triggerRect.right + 8
                  : 56,
          right: isRight ? (triggerRect ? window.innerWidth - triggerRect.left + 8 : 56) : 'auto',
          top: isBottom
            ? triggerRect
              ? triggerRect.bottom + 8
              : '50%'
            : triggerRect
              ? isHorizontal
                ? triggerRect.bottom + 8
                : triggerRect.top
              : '50%',
          bottom: 'auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
          padding: '6px 10px',
          background: 'var(--bg-elevated)',
          borderRadius: 8,
          boxShadow: 'var(--shadow-lg)',
          zIndex: 'var(--z-nav-popover)',
          border: '1px solid var(--border-subtle)',
          transformOrigin: 'top',
          maxWidth: 'calc(100vw - 32px)',
          maxHeight: `calc(100vh - ${SAFE_AREA_TOP} - ${SAFE_AREA_BOTTOM} - 32px)`,
          overflowY: 'auto',
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
            title={t('navigation:add_page_placeholder', { defaultValue: 'Choose icon' })}
          >
            {React.createElement(CUSTOM_ICON_MAP[selectedIconId], {
              size: 18,
              style: { color: 'var(--accent-primary)' },
            })}
          </button>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <input
              ref={inputRef}
              value={name}
              onChange={(e) => {
                setName(e.target.value.slice(0, 30));
                setRenameError(false);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleConfirm();
                if (e.key === 'Escape') handleCancel();
              }}
              maxLength={30}
              autoFocus
              className={styles.addPageInput}
              data-error={renameError || undefined}
            />
            <input
              value={description}
              onChange={(e) => setDescription(e.target.value.slice(0, 30))}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleConfirm();
                if (e.key === 'Escape') handleCancel();
              }}
              maxLength={30}
              placeholder={t('navigation:add_page_description_placeholder')}
              aria-label={t('navigation:add_page_description_placeholder')}
              className={styles.addPageInput}
              data-secondary
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
                  onClick={handleCancel}
                  className="interactive-accent-link"
                  style={{
                    fontSize: 'var(--text-badge)',
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    padding: 0,
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
            <IconCategoryPicker
              variant="inline"
              selectedIconId={selectedIconId}
              onSelect={(id) => {
                setSelectedIconId(id);
                setShowIconPicker(false);
              }}
            />
          </div>
        )}
      </div>
    ),
    document.body,
  );
}
