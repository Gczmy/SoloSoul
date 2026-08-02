import { useState, useRef, useCallback, useEffect, useLayoutEffect, type MouseEvent } from 'react';
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
  const { items, showSearch, setShowSearch } = useBoundNavActions();

  // ── Hover expand/collapse ──────────────────────────────────────
  const isHovering = useSidebarHoverStore((s) => s.isHovering);
  const setHovering = useSidebarHoverStore((s) => s.setHovering);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const isPluginPanelOpen = usePluginQuickStore((s) => s.isOpen);
  const [showQuickChat, setShowQuickChat] = useState(false);

  const verticalScrollTop = useSidebarHoverStore((s) => s.verticalScrollTop);
  const setVerticalScrollTop = useSidebarHoverStore((s) => s.setVerticalScrollTop);
  const contentRef = useRef<HTMLDivElement>(null);

  const isAnyCardOpen = isOcrCardOpen || isPluginPanelOpen || showSearch || showQuickChat;
  const isAnyCardOpenRef = useRef(isAnyCardOpen);
  isAnyCardOpenRef.current = isAnyCardOpen;
  const expanded = isHovering || isAnyCardOpen;

  // Collapse when mouse leaves the entire window (browser/webview may not fire
  // mouseleave on the wrapper in this case)
  useEffect(() => {
    const handleDocMouseLeave = () => {
      if (!isAnyCardOpenRef.current) setHovering(false);
    };
    document.documentElement.addEventListener('mouseleave', handleDocMouseLeave);
    return () => document.documentElement.removeEventListener('mouseleave', handleDocMouseLeave);
  }, [setHovering]);

  // Restore scroll position when expanded (useLayoutEffect to avoid flash before paint)
  useLayoutEffect(() => {
    if (expanded && contentRef.current) {
      contentRef.current.scrollTop = verticalScrollTop;
    }
  }, [expanded, verticalScrollTop]);

  // Save scroll position on every scroll
  const handleContentScroll = useCallback(() => {
    if (contentRef.current) {
      setVerticalScrollTop(contentRef.current.scrollTop);
    }
  }, [setVerticalScrollTop]);

  // Suppress name card tooltips during the 180ms expand CSS transition (buttons moving → flicker).
  // Use setTimeout rather than onTransitionEnd because React re-renders during the animation
  // can cause onTransitionEnd not to fire, leaving pointer-events permanently disabled.
  const [isTransitioning, setIsTransitioning] = useState(false);
  useEffect(() => {
    if (expanded) {
      setIsTransitioning(true);
      const timer = setTimeout(() => setIsTransitioning(false), 200);
      return () => clearTimeout(timer);
    } else {
      setIsTransitioning(false);
    }
  }, [expanded]);

  const handleMouseEnter = useCallback(() => {
    // 触屏设备不触发展开（Android WebView hover 会粘住）
    if (!supportsHover()) return;
    setHovering(true);
  }, [setHovering]);
  const handleMouseLeave = useCallback(
    (e: MouseEvent) => {
      if (wrapperRef.current && !wrapperRef.current.contains(e.relatedTarget as Node)) {
        const relatedTarget = e.relatedTarget as HTMLElement | null;
        const addPageZone = relatedTarget?.closest('[data-add-page-zone="true"]');
        if (addPageZone) {
          // Moving to AddPageButton: register one-time mouseleave to collapse
          // when mouse leaves the add page zone (unless re-entering wrapper).
          const handleLeaveAddPage = (leaveEvent: globalThis.MouseEvent) => {
            const leaveTarget = leaveEvent.relatedTarget as HTMLElement | null;
            if (wrapperRef.current?.contains(leaveTarget as Node)) return;
            if (!isAnyCardOpenRef.current) setHovering(false);
          };
          (addPageZone as HTMLElement).addEventListener('mouseleave', handleLeaveAddPage, {
            once: true,
          });
        } else if (!isAnyCardOpen) {
          setHovering(false);
        }
      }
    },
    [isAnyCardOpen, setHovering],
  );

  // ── Card positioning hooks ─────────────────────────────────────
  const ocrQuickScanPlacement =
    sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const pluginQuickPanelPlacement =
    sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const aiQuickChatPlacement =
    sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const { ocrButtonRef, quickScanPos } = useOcrQuickScan(560, ocrQuickScanPlacement);
  const { pluginButtonRef, quickPanelPos } = usePluginQuickPanel(560, pluginQuickPanelPlacement);
  const { aiButtonRef, quickChatPos, updateQuickChatPos } = useAiQuickChat(
    520,
    aiQuickChatPlacement,
  );

  // AI chat uses local state (not Zustand), so trigger position update and
  // attach scroll/resize listeners manually (hook's internal useEffect never
  // fires because its internal showQuickChat is always false here).
  useEffect(() => {
    if (!showQuickChat) return;
    updateQuickChatPos();
    window.addEventListener('scroll', updateQuickChatPos, true);
    window.addEventListener('resize', updateQuickChatPos);
    return () => {
      window.removeEventListener('scroll', updateQuickChatPos, true);
      window.removeEventListener('resize', updateQuickChatPos);
    };
  }, [showQuickChat, updateQuickChatPos]);

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
      quickChat: aiQuickChatPlacement,
      quickScan: ocrQuickScanPlacement,
      pluginPanel: pluginQuickPanelPlacement,
    },
  });

  return (
    <div
      ref={wrapperRef}
      className={styles.foldableWrapper}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {/* Arrow toggle — full-size button */}
      <div className={styles.arrowToggle}>
        <ChevronUp
          size={ICON_SIZE.xl}
          className={`${styles.arrowIcon} ${expanded ? styles.arrowIconExpanded : ''}`}
        />
      </div>

      {/* Foldable button area — always rendered for smooth CSS transition */}
      <div className={`${styles.foldableArea} ${expanded ? styles.foldableAreaOpen : ''}`}>
        <div
          ref={contentRef}
          className={styles.foldableContent}
          onScroll={handleContentScroll}
          style={isTransitioning ? { pointerEvents: 'none' as const } : undefined}
        >
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
