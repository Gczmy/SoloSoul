import React, {
  useState,
  useRef,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useContext,
} from 'react';
import { createPortal } from 'react-dom';
import { Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './SideNavigation.module.css';
import navStyles from './NavButton.module.css';
import { DesktopSidebarContext } from './DesktopSidebarContext';
import { DEFAULT_CUSTOM_ICON } from '@/lib/pageIcons';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import { useToastError } from '@/hooks/useToastError';
import { useAddPageForm } from '@/hooks/useAddPageForm';
import { AddPagePopover } from './AddPagePopover';
import { ICON_SIZE, SAFE_AREA_TOP } from '@/lib/constants';
import type { CustomPage } from '@/stores/settingsStore';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';

/** 预留的顶部空间：移动 AppBar (48px) + 安全区 + 8px 边距；
 * 桌面端 AppBar 为 56px，64px 也能满足。 */
const TOP_RESERVED_OFFSET = 64;
const MOBILE_APP_BAR_HEIGHT = 48;
const SIDEBAR_POPOVER_HEIGHT = 480;
const POPOVER_BOTTOM_MARGIN = 16;

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
  const sidebarExpanded = useContext(DesktopSidebarContext) && !isHorizontal;
  // 桌面直接复用普通导航按钮；移动底栏保留其显式传入的布局样式。
  const useNavStyle = !className && !buttonClassName;
  const isBottom = position === 'bottom';
  const isRight = position === 'right';
  const titlebarHeight = useNativeWindowStore((state) => state.titlebarHeight);
  const topReserved = Math.max(TOP_RESERVED_OFFSET, titlebarHeight + 8);
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
  // 侧栏卡片优先保留完整表单与图标区域；下方不足时向上避让，而非压缩成窄条。
  const sidebarPopoverHeight = Math.min(
    SIDEBAR_POPOVER_HEIGHT,
    Math.max(0, viewportHeight - topReserved - POPOVER_BOTTOM_MARGIN),
  );
  const popoverTop = useMemo(() => {
    if (isBottom || !buttonRect) return topReserved;
    if (isHorizontal) return Math.max(buttonRect.bottom + 8, topReserved);
    return Math.max(
      topReserved,
      Math.min(buttonRect.top, viewportHeight - POPOVER_BOTTOM_MARGIN - sidebarPopoverHeight),
    );
  }, [buttonRect, isHorizontal, isBottom, topReserved, viewportHeight, sidebarPopoverHeight]);

  // Compute max height for icon picker scroll area based on available viewport space
  const scrollMaxHeight = useMemo(() => {
    // 侧栏/底栏使用固定可用高度与 flex，只有顶部工具条仍限制图标区最大高度。
    if (isBottom || !isHorizontal) return undefined;
    // name input(~40) + optional description input(~40) + gap/padding(~24) + label(~14)
    const nonInputHeight = showDescription ? 118 : 72;
    // 底部留 16px 边距；icon 滚动区最小保留 48px（窗口极矮时宁可图标区滚动）
    const available = viewportHeight - popoverTop - POPOVER_BOTTOM_MARGIN - nonInputHeight;
    return Math.max(48, Math.min(280, available));
  }, [popoverTop, showDescription, isBottom, isHorizontal, viewportHeight]);

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

  // 首帧完成定位；打开期间随窗口缩放、侧栏宽度及滚动更新锚点，避免使用旧按钮坐标。
  useLayoutEffect(() => {
    if (!isCreating) return;
    const button = buttonRef.current;
    if (!button) return;
    const updatePosition = () => {
      setViewportHeight(window.innerHeight);
      const rect = button.getBoundingClientRect();
      setButtonRect((previous) =>
        previous &&
        previous.x === rect.x &&
        previous.y === rect.y &&
        previous.width === rect.width &&
        previous.height === rect.height
          ? previous
          : rect,
      );
    };
    updatePosition();
    const observer = new ResizeObserver(updatePosition);
    observer.observe(button);
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
    };
  }, [isCreating, sidebarExpanded, position]);

  const nameCard =
    isHovered && !isCreating && !sidebarExpanded ? (
      <div
        className={isHorizontal ? navStyles.nameCardPortalHorizontal : navStyles.nameCardPortal}
        style={{
          position: 'fixed',
          ...cardStyle,
          zIndex: 200,
        }}
        role="tooltip"
        data-macos-glass="tooltip"
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
        className={
          useNavStyle
            ? `${navStyles.navItemWrapper} ${sidebarExpanded ? navStyles.expanded : !isHorizontal ? navStyles.compactLabels : ''}`
            : undefined
        }
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        style={className || !isHorizontal ? undefined : { width: 40, height: 40 }}
      >
        <button
          type="button"
          ref={buttonRef}
          className={
            useNavStyle ? navStyles.navButton : `${styles.addPageButton} ${buttonClassName || ''}`
          }
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
          <Plus size={useNavStyle ? 20 : ICON_SIZE.xl} />
          {showLabel && (
            <span className={useNavStyle ? navStyles.label : styles.addPageLabel}>
              {t('add_page')}
            </span>
          )}
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
              zIndex: 'var(--z-nav-popover)',
              border: '1px solid var(--border-subtle)',
              transformOrigin: 'top',
              maxWidth: 'calc(100vw - 32px)',
              height: isHorizontal ? undefined : sidebarPopoverHeight,
              // 最大高度锚定卡片顶部：100vh - top - 16px 底部边距，保证
              // 卡片底部始终位于窗口底部之上（修复：侧边栏靠下时卡片底部超屏）
              maxHeight: isBottom ? undefined : `calc(100vh - ${popoverTop}px - 16px)`,
              overflowY: isBottom || !isHorizontal ? 'hidden' : 'auto',
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
