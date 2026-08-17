import React, { useState, useRef, useCallback, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { CustomPage } from '@/stores/settingsStore';
import styles from './SideNavigation.module.css';
import { DEFAULT_CUSTOM_ICON, type CustomIconId } from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS } from './useNavigationItems';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import { IconCategoryPicker } from './IconCategoryPicker';
import { ICON_SIZE, SAFE_AREA_TOP } from '@/lib/constants';

/** 预留的顶部空间：移动 AppBar (48px) + 安全区 + 8px 边距；
 * 桌面端 AppBar 为 56px，64px 也能满足。 */
const TOP_RESERVED_OFFSET = 64;
const MOBILE_APP_BAR_HEIGHT = 48;

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
  const [viewportHeight, setViewportHeight] = useState(
    typeof window !== 'undefined' ? window.innerHeight : 0,
  );
  const isSmallWindow = viewportHeight < 500;
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [nameError, setNameError] = useState<'empty' | 'duplicate' | null>(null);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(DEFAULT_CUSTOM_ICON);
  const [buttonRect, setButtonRect] = useState<DOMRect | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  /**
   * 弹出卡片顶部坐标（px）。非 bottom 模式：从按钮下方（horizontal）或按钮
   * 顶部对齐（侧边栏）向下展开，并保证不低于预留顶栏高度。bottom 模式不参与
   * （顶部固定为移动 AppBar + 安全区，无需此处计算）。
   */
  const popoverTop = useMemo(() => {
    if (isBottom || !buttonRect) return TOP_RESERVED_OFFSET;
    return Math.max(isHorizontal ? buttonRect.bottom + 8 : buttonRect.top, TOP_RESERVED_OFFSET);
  }, [buttonRect, isHorizontal, isBottom]);

  // Compute max height for icon picker scroll area based on available viewport space
  const scrollMaxHeight = useMemo(() => {
    if (isBottom) return undefined;
    // name input(~40) + optional description input(~40) + gap/padding(~24) + label(~14)
    const nonInputHeight = showDescription ? 118 : 72;
    // 底部留 16px 边距；icon 滚动区最小保留 48px（窗口极矮时宁可图标区滚动）
    const available = window.innerHeight - popoverTop - 16 - nonInputHeight;
    return Math.max(48, Math.min(280, available));
  }, [popoverTop, showDescription, isBottom]);

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
    setNameError(null);
    setSelectedIconId(DEFAULT_CUSTOM_ICON);
  }, []);

  const handleConfirm = useCallback(
    (isExplicit = false) => {
      const trimmed = name.trim();
      if (!trimmed || !currentAccount) {
        if (isExplicit) {
          setNameError('empty');
        } else {
          handleCancel();
        }
        return;
      }
      // Check for duplicate page names
      const store = useSettingsStore.getState();
      const existingNames = [
        ...SYSTEM_PAGE_KEYS.map((k) => t(k)),
        ...store.settings.customPages.filter((p) => !p.deletedAt).map((p) => p.name),
      ];
      if (existingNames.some((n) => n.toLowerCase() === trimmed.toLowerCase())) {
        setNameError('duplicate');
        return;
      }
      const trimmedDesc = description.trim();
      addCustomPage(currentAccount.id, trimmed, selectedIconId, trimmedDesc || undefined).then(
        (page) => {
          onCreate(page);
        },
      );
      handleCancel();
    },
    [name, description, selectedIconId, currentAccount, addCustomPage, onCreate, t, handleCancel],
  );

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
        handleConfirm(false);
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

  // Hover name card（共享定位 hook，同 NavButton 的 portal 模式）
  const wrapperRef = useRef<HTMLDivElement>(null);
  const { cardStyle, isHovered, handleMouseEnter, handleMouseLeave } = useHoverCardPosition(
    wrapperRef,
    { isHorizontal, isBottom, isRight },
  );

  // Track viewport height so small-window detection stays reactive to resize/rotate
  useEffect(() => {
    const handleResize = () => setViewportHeight(window.innerHeight);
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

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
        isCreating && (
          <div
            ref={popoverRef}
            className={styles.addPagePopover}
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
                ? `calc(${MOBILE_APP_BAR_HEIGHT}px + ${SAFE_AREA_TOP} + 8px)`
                : popoverTop,
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
              // 最大高度锚定卡片顶部：100vh - top - 16px 底部边距，保证
              // 卡片底部始终位于窗口底部之上（修复：侧边栏靠下时卡片底部超屏）
              maxHeight: isBottom ? undefined : `calc(100vh - ${popoverTop}px - 16px)`,
              overflowY: isBottom ? 'hidden' : 'auto',
            }}
          >
            {/* Name input */}
            <input
              ref={inputRef}
              value={name}
              onChange={(e) => {
                setName(e.target.value.slice(0, 20));
                setNameError(null);
              }}
              onBlur={(e) => {
                // Only confirm if the blur is not caused by clicking inside the popover
                if (popoverRef.current && !popoverRef.current.contains(e.relatedTarget as Node)) {
                  handleConfirm(false);
                }
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleConfirm(true);
                if (e.key === 'Escape') handleCancel();
              }}
              placeholder={t('add_page_placeholder')}
              maxLength={20}
              autoFocus
              aria-label={t('add_page_placeholder')}
              className={styles.addPageInput}
              data-error={nameError ? 'true' : undefined}
              style={{ flexShrink: 0 }}
            />
            {showDescription && (
              <input
                value={description}
                onChange={(e) => setDescription(e.target.value.slice(0, 30))}
                onBlur={(e) => {
                  if (popoverRef.current && !popoverRef.current.contains(e.relatedTarget as Node)) {
                    handleConfirm(false);
                  }
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleConfirm(true);
                  if (e.key === 'Escape') handleCancel();
                }}
                placeholder={t('add_page_description_placeholder')}
                maxLength={30}
                aria-label={t('add_page_description_placeholder')}
                className={styles.addPageInput}
                data-secondary
                style={{ flexShrink: 0 }}
              />
            )}
            {nameError && (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 8,
                  flexShrink: 0,
                }}
              >
                <span
                  style={{
                    fontSize: 'var(--text-badge)',
                    color: '#e74c3c',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {nameError === 'empty' ? t('page_name_required') : t('page_name_exists')}
                </span>
                <button onClick={handleCancel} className={styles.cancelTextBtn}>
                  {t('common:cancel')}
                </button>
              </div>
            )}

            {/* Icon picker with category sections (scrollable) */}
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 6,
                ...(isBottom && {
                  flex: '1 1 auto',
                  minHeight: isSmallWindow ? 80 : 120,
                  overflow: 'hidden',
                }),
              }}
            >
              <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                {t('select_icon')}
              </span>
              <div
                style={{
                  maxHeight: isBottom ? undefined : scrollMaxHeight,
                  overflowY: 'auto',
                  overflowX: 'hidden',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 10,
                  ...(isBottom && { flex: '1 1 auto', minHeight: 0 }),
                }}
              >
                <IconCategoryPicker selectedIconId={selectedIconId} onSelect={setSelectedIconId} />
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
                flexShrink: 0,
                marginTop: isBottom ? 'auto' : undefined,
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
              </button>{' '}
              <button
                onClick={() => handleConfirm(true)}
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
          </div>
        ),
        document.body,
      )}
    </div>
  );
}
