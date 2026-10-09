import {
  useState,
  useRef,
  useCallback,
  useEffect,
  useLayoutEffect,
  useContext,
  useId,
  type CSSProperties,
} from 'react';
import { useTranslation } from 'react-i18next';
import { DesktopSidebarContext } from './DesktopSidebarContext';
import { useLocation, useNavigate } from 'react-router-dom';
import { ChevronUp } from 'lucide-react';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import { useSidebarHoverStore } from '@/stores/sidebarHoverStore';
import { useNavButtonCards } from './navButtonCards';
import { supportsHover } from '@/lib/platform';
import {
  useBoundNavActions,
  useAiQuickChat,
  useOcrQuickScan,
  usePluginQuickPanel,
  CARD_ACTION_IDS,
} from './useNavigationItems';
import styles from './SideNavigation.module.css';
import type { NavPosition } from './NavButton';
import { ICON_SIZE } from '@/lib/constants';

interface SecondaryActionBarProps {
  sidebarPosition: NavPosition;
  isHorizontal: boolean;
}

export function SecondaryActionBar({
  sidebarPosition,
  isHorizontal: _isHorizontal,
}: SecondaryActionBarProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const sidebarExpanded = useContext(DesktopSidebarContext);
  const { t } = useTranslation('navigation');
  const { items, showSearch, setShowSearch } = useBoundNavActions();

  // ── Hover expand/collapse ──────────────────────────────────────
  const isHovering = useSidebarHoverStore((s) => s.isHovering);
  const setHovering = useSidebarHoverStore((s) => s.setHovering);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const toolsId = useId();
  const [keyboardOpen, setKeyboardOpen] = useState(false);
  const [availableHeight, setAvailableHeight] = useState(0);
  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const isPluginPanelOpen = usePluginQuickStore((s) => s.isOpen);
  const cardPlacement =
    sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const { showQuickChat, setShowQuickChat, aiButtonRef, quickChatPos } = useAiQuickChat(
    520,
    cardPlacement,
  );

  const verticalScrollTop = useSidebarHoverStore((s) => s.verticalScrollTop);
  const setVerticalScrollTop = useSidebarHoverStore((s) => s.setVerticalScrollTop);
  const contentRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const isAnyCardOpen = isOcrCardOpen || isPluginPanelOpen || showSearch || showQuickChat;
  const expanded = isHovering || keyboardOpen || isAnyCardOpen;

  // 展开和折叠侧栏共用向上浮出的工具菜单，不挤压分类区或移动触发按钮。
  // 高度以实际导航起点和工具入口之间的空间为准，小窗口才滚动。
  const measureTools = useCallback(() => {
    const wrapper = wrapperRef.current;
    const nav = wrapper?.closest('nav');
    const menu = menuRef.current;
    if (!wrapper || !nav || !menu) return;
    const brand = nav.querySelector(`.${styles.brandHeader}`);
    // 折叠后品牌行不再存在，仍需避让侧栏上方的原生交通灯与顶部留白。
    const top =
      brand?.getBoundingClientRect().bottom ??
      nav.getBoundingClientRect().top + parseFloat(getComputedStyle(nav).paddingTop || '0');
    const height = Math.max(0, Math.floor(wrapper.getBoundingClientRect().top - top - 8));
    const coverHeight = `${menu.getBoundingClientRect().height}px`;
    // 集中读取几何后再写入；相同裁剪高度不反复使样式失效。
    setAvailableHeight(height);
    if (nav.style.getPropertyValue('--tools-cover-height') !== coverHeight) {
      nav.style.setProperty('--tools-cover-height', coverHeight);
    }
  }, []);

  useLayoutEffect(() => {
    const wrapper = wrapperRef.current;
    const nav = wrapper?.closest('nav');
    const menu = menuRef.current;
    if (!wrapper || !nav || !menu) return;
    // 菜单常驻DOM，开关和侧栏折叠无需重建监听或暂时清空裁剪高度。
    const observer = new ResizeObserver(measureTools);
    observer.observe(nav);
    observer.observe(wrapper);
    observer.observe(menu);
    window.addEventListener('resize', measureTools);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', measureTools);
      nav.style.removeProperty('--tools-cover-height');
    };
  }, [measureTools]);

  useLayoutEffect(measureTools, [measureTools, sidebarExpanded, expanded, sidebarPosition]);

  useLayoutEffect(() => {
    if (!expanded && contentRef.current?.contains(document.activeElement)) {
      toggleRef.current?.focus();
    }
  }, [expanded]);

  // Collapse when mouse leaves the entire window (browser/webview may not fire
  // mouseleave on the wrapper in this case)
  useEffect(() => {
    const handleDocMouseLeave = () => {
      setHovering(false);
    };
    document.documentElement.addEventListener('mouseleave', handleDocMouseLeave);
    return () => document.documentElement.removeEventListener('mouseleave', handleDocMouseLeave);
  }, [setHovering]);

  useEffect(() => {
    if (!expanded) return;
    const handleOutsidePointer = (event: PointerEvent) => {
      if (wrapperRef.current?.contains(event.target as Node) || isAnyCardOpen) return;
      setHovering(false);
      setKeyboardOpen(false);
    };
    document.addEventListener('pointerdown', handleOutsidePointer);
    return () => document.removeEventListener('pointerdown', handleOutsidePointer);
  }, [expanded, isAnyCardOpen, setHovering]);

  // Restore scroll position when expanded (useLayoutEffect to avoid flash before paint)
  useLayoutEffect(() => {
    if (expanded && contentRef.current) {
      contentRef.current.scrollTop = verticalScrollTop;
    }
  }, [expanded, verticalScrollTop]);

  // P216: 折叠时保存滚动位置（而非每帧 onScroll 写 store）。
  // foldableContent 常驻 DOM（隐藏时保留几何尺寸），scrollTop 不丢失，
  // 故只需在展开→折叠的瞬间持久化一次，重新展开时由上面的 useLayoutEffect 恢复。
  // 注意：必须只在 true→false 转换时保存——初始挂载（未展开）时 DOM 的 scrollTop 为 0，
  // 若直接写会清掉上次会话持久化的滚动位置（sidebarHoverStore 跨路由持久化的目的）。
  const prevExpandedRef = useRef(expanded);
  useEffect(() => {
    const wasExpanded = prevExpandedRef.current;
    prevExpandedRef.current = expanded;
    if (wasExpanded && !expanded && contentRef.current) {
      setVerticalScrollTop(contentRef.current.scrollTop);
    }
  }, [expanded, setVerticalScrollTop]);

  // 卸载兜底：展开态下直接切换路由时组件卸载，保存当前滚动位置。
  useEffect(() => {
    const el = contentRef.current;
    return () => {
      if (el) {
        setVerticalScrollTop(el.scrollTop);
      }
    };
  }, [setVerticalScrollTop]);

  const handleMouseEnter = useCallback(
    (event: globalThis.MouseEvent) => {
      if (!supportsHover() || !wrapperRef.current?.contains(event.target as Node)) return;
      setHovering(true);
    },
    [setHovering],
  );
  const handleMouseLeave = useCallback(
    (e: globalThis.MouseEvent) => {
      if (
        wrapperRef.current &&
        !(e.relatedTarget instanceof Node && wrapperRef.current.contains(e.relatedTarget))
      ) {
        setHovering(false);
      }
    },
    [setHovering],
  );

  // 原生 enter/leave 以实际 DOM 范围判定；React 的合成事件会把 Portal
  // 卡片也当作工具区后代，导致鼠标已移出菜单但悬停状态仍然保留。
  useEffect(() => {
    const wrapper = wrapperRef.current;
    if (!wrapper) return;
    wrapper.addEventListener('mouseenter', handleMouseEnter);
    wrapper.addEventListener('mouseleave', handleMouseLeave);
    return () => {
      wrapper.removeEventListener('mouseenter', handleMouseEnter);
      wrapper.removeEventListener('mouseleave', handleMouseLeave);
    };
  }, [handleMouseEnter, handleMouseLeave]);

  const { ocrButtonRef, quickScanPos } = useOcrQuickScan(560, cardPlacement);
  const { pluginButtonRef, quickPanelPos } = usePluginQuickPanel(560, cardPlacement);

  // ── Render helpers (shared with TopFunctionBar) ────────────────
  const { renderButtonWithCard, renderPlainButton } = useNavButtonCards({
    position: sidebarPosition,
    navigate,
    location,
    showSearch,
    setShowSearch,
    showQuickChat,
    setShowQuickChat,
    pluginButtonRef,
    ocrButtonRef,
    aiButtonRef,
    quickChatPos,
    quickScanPos,
    quickPanelPos,
    placements: {
      quickChat: cardPlacement,
      quickScan: cardPlacement,
      pluginPanel: cardPlacement,
    },
  });

  return (
    <div
      ref={wrapperRef}
      className={styles.foldableWrapper}
      style={{ '--tools-available-height': `${availableHeight}px` } as CSSProperties}
      onBlur={(event) => {
        if (
          !(
            event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)
          )
        ) {
          setKeyboardOpen(false);
          // 搜索卡片自动聚焦输入框时，鼠标仍可能停在工具按钮上。
          // Portal 的焦点变化不清除实际悬停；返回其他导航按钮时才释放菜单。
          if (
            event.relatedTarget instanceof Node &&
            event.currentTarget.closest('nav')?.contains(event.relatedTarget)
          ) {
            setHovering(false);
          }
        }
      }}
      onKeyDown={(event) => {
        // Portal 卡片的按键由卡片自身处理。
        if (!event.currentTarget.contains(event.target as Node)) return;
        if (event.key === 'Tab' && expanded) setKeyboardOpen(true);
        if (event.key === 'Escape' && !isAnyCardOpen) {
          event.stopPropagation();
          toggleRef.current?.focus();
          setHovering(false);
          setKeyboardOpen(false);
        }
      }}
    >
      {/* Arrow toggle — full-size button */}
      <button
        ref={toggleRef}
        type="button"
        className={styles.arrowToggle}
        aria-label={t('sidebar_tools')}
        aria-expanded={expanded}
        aria-controls={toolsId}
        onClick={() => {
          // 鼠标悬停已打开时，点击保留展开；再次点击可收起。
          // 键盘直接激活也能打开，无需依赖 hover。
          setKeyboardOpen(!keyboardOpen);
          setHovering(false);
        }}
      >
        <ChevronUp
          size={sidebarExpanded ? ICON_SIZE.sm : ICON_SIZE.xl}
          className={`${styles.arrowIcon} ${expanded ? styles.arrowIconExpanded : ''}`}
        />
        <span className={styles.compactActionLabel}>{t('sidebar_tools')}</span>
      </button>

      {/* Foldable button area — always rendered for smooth CSS transition */}
      <div
        ref={menuRef}
        id={toolsId}
        role="group"
        aria-label={t('sidebar_tools')}
        data-sidebar-tools
        data-open={expanded}
        data-macos-glass="sidebar-menu"
        className={`${styles.foldableArea} ${expanded ? styles.foldableAreaOpen : ''}`}
        inert={!expanded}
      >
        <div ref={contentRef} className={styles.foldableContent}>
          {items.map((item) => {
            const isCardButton = (CARD_ACTION_IDS as readonly string[]).includes(item.iconKey);
            if (isCardButton) {
              const cardEl = renderButtonWithCard(item);
              if (cardEl) return cardEl;
            }
            return renderPlainButton(item);
          })}
        </div>
      </div>
    </div>
  );
}
