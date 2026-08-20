import React, { useState, useRef, useCallback, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './SideNavigation.module.css';
import { DEFAULT_CUSTOM_ICON } from '@/lib/pageIcons';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import { useToastError } from '@/hooks/useToastError';
import { useAddPageForm } from '@/hooks/useAddPageForm';
import { AddPagePopover } from './AddPagePopover';
import { ICON_SIZE, SAFE_AREA_TOP } from '@/lib/constants';
import type { CustomPage } from '@/stores/settingsStore';

/** 预留的顶部空间：移动 AppBar (48px) + 安全区 + 8px 边距；
 * 桌面端 AppBar 为 56px，64px 也能满足。 */
const TOP_RESERVED_OFFSET = 64;
const MOBILE_APP_BAR_HEIGHT = 48;

// =============================================================================
// AddPageButton — "+" button with popover for name + icon selection
// （P021d 拆分：表单状态 → useAddPageForm，弹层 UI → AddPagePopover）
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
  const { onError } = useToastError();
  const form = useAddPageForm({ onCreate, t, onError });
  // 解构出子 hook 的稳定函数，供本组件 useCallback 依赖使用（避免每次渲染新建对象）
  const { handleCancel: resetForm, handleConfirm: confirmForm } = form;

  // 关闭弹层并重置表单
  const handleCancel = useCallback(() => {
    setIsCreating(false);
    resetForm();
  }, [resetForm]);

  // 确认创建：错误路径（显式空名称/重名）留在弹层，其余关闭
  const handleConfirm = useCallback(
    (isExplicit = false) => {
      if (confirmForm(isExplicit)) setIsCreating(false);
    },
    [confirmForm],
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
            form.setSelectedIconId(DEFAULT_CUSTOM_ICON);
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
          <AddPagePopover
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
            popoverRef={popoverRef}
            inputRef={inputRef}
            name={form.name}
            onNameChange={(v) => {
              form.setName(v);
              form.setNameError(null);
            }}
            description={form.description}
            onDescriptionChange={form.setDescription}
            nameError={form.nameError}
            selectedIconId={form.selectedIconId}
            onSelectIcon={form.setSelectedIconId}
            onConfirm={handleConfirm}
            onCancel={handleCancel}
            showDescription={!!showDescription}
            scrollMaxHeight={scrollMaxHeight}
            isBottom={isBottom}
            isSmallWindow={isSmallWindow}
            t={t}
          />
        ),
        document.body,
      )}
    </div>
  );
}
