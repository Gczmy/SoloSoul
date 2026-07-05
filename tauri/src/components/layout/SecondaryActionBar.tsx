import { useState, useRef, useCallback, useEffect, useLayoutEffect, type MouseEvent } from 'react';
import { createPortal } from 'react-dom';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ChevronUp } from 'lucide-react';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useSidebarHoverStore } from '@/stores/sidebarHoverStore';
import { NavButton } from './NavButton';
import { SearchPopover } from './SearchPopover';
import { AiQuickChatPopover } from './AiQuickChatPopover';
import { OcrQuickScanPopover } from './OcrQuickScanPopover';
import { PluginQuickPanel } from '@/components/plugin/PluginQuickPanel';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import {
  useBoundNavActions,
  useAiQuickChat,
  useOcrQuickScan,
  usePluginQuickPanel,
  CARD_ACTION_IDS,
  NavAction,
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
  const { t } = useTranslation('navigation');

  const { items, showSearch, setShowSearch } = useBoundNavActions();

  // ── Hover expand/collapse ──────────────────────────────────────
  const isHovering = useSidebarHoverStore((s) => s.isHovering);
  const setHovering = useSidebarHoverStore((s) => s.setHovering);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const isPluginPanelOpen = usePluginQuickStore((s) => s.isOpen);
  const aiChatMode = useSettingsStore((s) => s.settings.sidebarButtonModes['ai_chat']);
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

  const handleMouseEnter = useCallback(() => setHovering(true), [setHovering]);
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
          (addPageZone as HTMLElement).addEventListener('mouseleave', handleLeaveAddPage, { once: true });
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
  const { aiButtonRef, quickChatPos } = useAiQuickChat(520, aiQuickChatPlacement);

  // ── Render helpers ─────────────────────────────────────────────
  const renderButtonWithCard = (item: (typeof items)[number]) => {
    // Page mode (type === 'link'): render as plain navigation button.
    // Note: ai_chat always returns type: 'link' even in card mode, so exclude it.
    if (item.type === 'link' && item.iconKey !== 'ai_chat') {
      return renderPlainButton(item);
    }

    if (item.iconKey === 'plugins') {
      return (
        <div ref={pluginButtonRef} key="plugins" data-plugin-button="true">
          <NavButton
            Icon={PAGE_ICON_MAP[item.iconKey]}
            label={t(item.labelKey)}
            isActive={isPluginPanelOpen}
            onClick={item.type === 'action' ? (item as NavAction).action : () => {}}
            position={sidebarPosition}
          />
          {isPluginPanelOpen &&
            createPortal(
              <PluginQuickPanel
                position={quickPanelPos}
                onClose={() => usePluginQuickStore.getState().setOpen(false)}
                placement={pluginQuickPanelPlacement}
              />,
              document.body,
            )}
        </div>
      );
    }
    if (item.iconKey === 'ocr') {
      return (
        <div ref={ocrButtonRef} key="ocr" data-ocr-button="true">
          <NavButton
            Icon={PAGE_ICON_MAP[item.iconKey]}
            label={t(item.labelKey)}
            isActive={isOcrCardOpen}
            onClick={item.type === 'action' ? (item as NavAction).action : () => {}}
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
    if (item.iconKey === 'search') {
      return (
        <div key="search" style={{ position: 'relative' }}>
          <NavButton
            Icon={PAGE_ICON_MAP[item.iconKey]}
            label={t(item.labelKey)}
            isActive={showSearch}
            onClick={item.type === 'action' ? (item as NavAction).action : () => {}}
            position={sidebarPosition}
          />
          {showSearch &&
            createPortal(<SearchPopover onClose={() => setShowSearch(false)} />, document.body)}
        </div>
      );
    }
    // ai_chat
    if (item.iconKey === 'ai_chat') {
      return (
        <div ref={aiButtonRef} key="ai_chat" data-ai-button="true">
          <NavButton
            Icon={PAGE_ICON_MAP[item.iconKey]}
            label={t(item.labelKey)}
            isActive={showQuickChat || location.pathname.startsWith('/llm-chat')}
            onClick={() => {
              if (aiChatMode === 'page') {
                navigate('/llm-chat');
              } else if (!location.pathname.startsWith('/llm-chat')) {
                setShowQuickChat((prev) => !prev);
              }
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
    return null;
  };

  const renderPlainButton = (item: (typeof items)[number]) => {
    if (item.type === 'action') {
      return (
        <NavButton
          key={item.iconKey}
          Icon={PAGE_ICON_MAP[item.iconKey]}
          label={t(item.labelKey)}
          onClick={item.action}
          position={sidebarPosition}
        />
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
  };

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
