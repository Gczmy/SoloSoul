import { useState, useRef, useCallback, useEffect, useLayoutEffect, type MouseEvent } from 'react';
import { createPortal } from 'react-dom';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ChevronRight } from 'lucide-react';
import styles from './TopFunctionBar.module.css';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { NavButton } from './NavButton';
import { RenameableNavButton, AddPageButton } from './SideNavigation';
import { AiQuickChatPopover } from './AiQuickChatPopover';
import { SearchPopover } from './SearchPopover';
import { OcrQuickScanPopover } from './OcrQuickScanPopover';
import { PluginQuickPanel } from '@/components/plugin/PluginQuickPanel';
import {
  useActiveCustomPages,
  useBoundNavActions,
  useAiQuickChat,
  useOcrQuickScan,
  usePluginQuickPanel,
  primaryItems,
  CARD_ACTION_IDS,
} from './useNavigationItems';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useVaultStore } from '@/stores/vaultStore';
import { useSidebarHoverStore } from '@/stores/sidebarHoverStore';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import type { CustomPage } from '@/stores/settingsStore';
import type { WheelEvent, UIEvent } from 'react';
import { ICON_SIZE } from '@/lib/iconSizes';


const FUNCTION_BAR_HEIGHT = 48;

export function TopFunctionBar({ sidebarPosition }: { sidebarPosition?: import('./NavButton').NavPosition }) {
  const POSITION: import('./NavButton').NavPosition = sidebarPosition === 'bottom' ? 'bottom' : 'top';
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('navigation');
  const activeCustomPages = useActiveCustomPages();
  const { items, showSearch, setShowSearch } = useBoundNavActions();
  const vaultLock = useVaultStore((s) => s.lock);

  // ── Card states ─────────────────────────────────────────────────
  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const isPluginPanelOpen = usePluginQuickStore((s) => s.isOpen);
  const aiChatMode = useSettingsStore((s) => s.settings.sidebarButtonModes['ai_chat']);

  // ── Card positioning hooks (single calls) ───────────────────────
  const {
    showQuickChat: quickChatFromHook,
    setShowQuickChat,
    aiButtonRef,
    quickChatPos,
  } = useAiQuickChat(520, 'bottom');
  const { ocrButtonRef, quickScanPos } = useOcrQuickScan(560, 'bottom');
  const { pluginButtonRef, quickPanelPos } = usePluginQuickPanel(560, 'bottom');

  const showQuickChat = quickChatFromHook;

  // ── Hover expand/collapse ──────────────────────────────────────
  const isHovering = useSidebarHoverStore((s) => s.isHovering);
  const setHovering = useSidebarHoverStore((s) => s.setHovering);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const horizontalScrollLeft = useSidebarHoverStore((s) => s.horizontalScrollLeft);
  const setHorizontalScrollLeft = useSidebarHoverStore((s) => s.setHorizontalScrollLeft);

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

  // Restore horizontal scroll position when expanded (useLayoutEffect to avoid flash before paint)
  useLayoutEffect(() => {
    if (expanded && funcScrollRef.current) {
      funcScrollRef.current.scrollLeft = horizontalScrollLeft;
    }
  }, [expanded, horizontalScrollLeft]);

  // Save horizontal scroll position on every scroll
  const handleFuncScroll = useCallback(
    (e: UIEvent<HTMLDivElement>) => {
      setHorizontalScrollLeft((e.target as HTMLDivElement).scrollLeft);
    },
    [setHorizontalScrollLeft],
  );

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
      // Only collapse if mouse actually left the wrapper (not a re-render artifact)
      if (wrapperRef.current && !wrapperRef.current.contains(e.relatedTarget as Node)) {
        if (!isAnyCardOpen) setHovering(false);
      }
    },
    [isAnyCardOpen, setHovering],
  );



  // ── Horizontal scroll (function buttons) ───────────────────────
  const funcScrollRef = useRef<HTMLDivElement>(null);

  const handleFuncWheel = useCallback((e: WheelEvent<HTMLDivElement>) => {
    const el = funcScrollRef.current;
    if (!el) return;
    const delta = e.deltaY + e.deltaX;
    if (delta === 0) return;
    e.preventDefault();
    el.scrollBy({ left: delta, behavior: 'auto' });
  }, []);

  // ── Page navigation horizontal scroll ──────────────────────────
  const navScrollRef = useRef<HTMLDivElement>(null);

  const handleNavWheel = useCallback((e: WheelEvent<HTMLDivElement>) => {
    const el = navScrollRef.current;
    if (!el) return;
    const delta = e.deltaY + e.deltaX;
    if (delta === 0) return;
    e.preventDefault();
    el.scrollBy({ left: delta, behavior: 'auto' });
  }, []);

  // ── Page navigation helpers ────────────────────────────────────
  const isWorkspaceSectionActive = (sectionPath: string): boolean => {
    if (location.pathname.startsWith('/workspace/custom/')) return false;
    if (!location.pathname.startsWith('/workspace')) return false;
    const section = sectionPath.split('section=')[1];
    if (!section) return !location.search.includes('section=');
    return location.search.includes(`section=${section}`);
  };

  const isCustomPageActive = (pageId: string): boolean => {
    return location.pathname === `/workspace/custom/${pageId}`;
  };

  const handleCustomPageNavigate = (page: CustomPage) => {
    navigate(`/workspace/custom/${page.id}`);
  };

  // ── Render helpers for function buttons ─────────────────────────
  const renderButtonWithCard = (item: typeof items[number]) => {
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
            onClick={item.type === 'action' ? (item as import('./useNavigationItems').NavAction).action : (() => {})}
            position={POSITION}
          />
          {isPluginPanelOpen &&
            createPortal(
              <PluginQuickPanel
                position={quickPanelPos}
                onClose={() => usePluginQuickStore.getState().setOpen(false)}
                placement="bottom"
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
            onClick={item.type === 'action' ? (item as import('./useNavigationItems').NavAction).action : (() => {})}
            position={POSITION}
          />
          {isOcrCardOpen &&
            createPortal(
              <OcrQuickScanPopover
                position={quickScanPos}
                onClose={() => useOcrScanStore.getState().setCardOpen(false)}
                placement="bottom"
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
            onClick={item.type === 'action' ? (item as import('./useNavigationItems').NavAction).action : (() => {})}
            position={POSITION}
          />
          {showSearch &&
            createPortal(
              <SearchPopover onClose={() => setShowSearch(false)} />,
              document.body,
            )}
        </div>
      );
    }
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
            position={POSITION}
          />
          {showQuickChat &&
            createPortal(
              <AiQuickChatPopover
                position={quickChatPos}
                onClose={() => setShowQuickChat(false)}
                placement="bottom"
              />,
              document.body,
            )}
        </div>
      );
    }
    return null;
  };

  const renderPlainButton = (item: typeof items[number]) => {
    if (item.type === 'action') {
      return (
        <NavButton
          key={item.iconKey}
          Icon={PAGE_ICON_MAP[item.iconKey]}
          label={t(item.labelKey)}
          onClick={item.action}
          position={POSITION}
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
        position={POSITION}
      />
    );
  };

  const isBottom = sidebarPosition === 'bottom';

  return (
    <header
      className={styles.functionBar}
      data-tauri-drag-region
      style={{
        height: FUNCTION_BAR_HEIGHT,
        top: isBottom ? 'auto' : 0,
        bottom: isBottom ? 0 : 'auto',
        borderBottom: isBottom ? 'none' : '1px solid var(--border-subtle)',
        borderTop: isBottom ? '1px solid var(--border-subtle)' : 'none',
      }}
    >
      {/* Left zone: logo + primary pages + custom pages */}
      <div className={styles.leftZone} data-tauri-drag-region="false">
        <ShieldLogo size={ICON_SIZE['3xl']} />

        <nav className={styles.primaryZone} aria-label={t('home')}>
          {/* HOME — always visible */}
          <NavButton
            path={primaryItems[0].path}
            Icon={PAGE_ICON_MAP[primaryItems[0].iconKey]}
            label={t(primaryItems[0].labelKey)}
            isActive={location.pathname === '/'}
            onClick={() => navigate(primaryItems[0].path)}
            position={POSITION}
          />

          {/* Scrollable zone: identity / travel / financial / professional + custom pages */}
          <div ref={navScrollRef} className={styles.scrollablePages} onWheel={handleNavWheel}>
            {primaryItems.slice(1).map((item) => (
              <NavButton
                key={item.path}
                path={item.path}
                Icon={PAGE_ICON_MAP[item.iconKey]}
                label={t(item.labelKey)}
                isActive={isWorkspaceSectionActive(item.path)}
                onClick={() => navigate(item.path)}
                position={POSITION}
              />
            ))}
            {activeCustomPages.map((page) => (
              <RenameableNavButton
                key={page.id}
                page={page}
                isActive={isCustomPageActive(page.id)}
                onClick={() => handleCustomPageNavigate(page)}
                position={POSITION}
              />
            ))}
          </div>
        </nav>
      </div>

      {/* Right zone: add page + foldable function buttons + lock + settings */}
      <div className={styles.rightZone} data-tauri-drag-region="false">
        <AddPageButton
          onCreate={(page) => navigate(`/workspace/custom/${page.id}`)}
          position={POSITION}
        />

        {/* Horizontal foldable area */}
        <div
          ref={wrapperRef}
          className={styles.horizontalFoldableWrapper}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          {/* Arrow toggle — full-size button */}
          <div className={styles.horizontalArrowToggle}>
            <ChevronRight
              size={ICON_SIZE.xl}
              className={`${styles.horizontalArrowIcon} ${!expanded ? styles.horizontalArrowIconExpanded : ''}`}
            />
          </div>

          {/* Scrollable function buttons — always rendered for smooth CSS transition */}
          <div className={`${styles.horizontalButtonArea} ${expanded ? styles.horizontalButtonAreaOpen : ''}`}>
            <div
              ref={funcScrollRef}
              className={styles.horizontalButtonScroll}
              onWheel={handleFuncWheel}
              onScroll={handleFuncScroll}
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

        {/* Lock — always fixed */}
        <NavButton
          Icon={PAGE_ICON_MAP.lock}
          label={t('lock_vault')}
          onClick={vaultLock}
          position={POSITION}
        />

        {/* Settings — always fixed */}
        <NavButton
          path="/settings"
          Icon={PAGE_ICON_MAP.settings}
          label={t('settings')}
          isActive={location.pathname.startsWith('/settings')}
          onClick={() => navigate('/settings')}
          position={POSITION}
        />
      </div>
    </header>
  );
}
