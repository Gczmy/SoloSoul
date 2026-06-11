import { createPortal } from 'react-dom';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import styles from './TopFunctionBar.module.css';
import { NavButton } from './NavButton';
import {
  RenameableNavButton,
  AddPageButton,
  AiQuickChatPopover,
} from './SideNavigation';
import { SearchPopover } from './SearchPopover';
import {
  useActiveCustomPages,
  useBoundNavActions,
  useAiQuickChat,
  primaryItems,
} from './useNavigationItems';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import type { CustomPage } from '@/stores/settingsStore';

const FUNCTION_BAR_HEIGHT = 48;
const POSITION: import('./NavButton').NavPosition = 'top';

export function TopFunctionBar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('navigation');
  const activeCustomPages = useActiveCustomPages();
  const { items, showSearch, setShowSearch } = useBoundNavActions();
  const { showQuickChat, setShowQuickChat, aiButtonRef, quickChatPos } = useAiQuickChat(520, 'bottom');

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

  return (
    <header
      className={styles.functionBar}
      data-tauri-drag-region
      style={{ height: FUNCTION_BAR_HEIGHT }}
    >
      {/* Left zone: logo + primary pages + custom pages + add page */}
      <div className={styles.leftZone} data-tauri-drag-region="false">
        <div className={styles.logo}>S</div>

        <nav className={styles.primaryZone} aria-label={t('home')}>
          {/* HOME — always visible (static left) */}
          <NavButton
            path={primaryItems[0].path}
            Icon={PAGE_ICON_MAP[primaryItems[0].iconKey]}
            label={t(primaryItems[0].labelKey)}
            isActive={location.pathname === '/'}
            onClick={() => navigate(primaryItems[0].path)}
            position={POSITION}
          />

          {/* Scrollable zone: identity / travel / financial / professional + custom pages (excludes AddPageButton) */}
          <div className={styles.scrollablePages}>
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

      {/* Right zone: add page + secondary actions */}
      <div className={styles.rightZone} data-tauri-drag-region="false">
        <AddPageButton
          onCreate={(page) => navigate(`/workspace/custom/${page.id}`)}
          position={POSITION}
        />
        {items.map((item, i) => {
          if (item.type === 'action') {
            const isSearch = item.iconKey === 'search';
            return (
              <div key={`action-${i}`} style={{ position: 'relative' }}>
                <NavButton
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  onClick={item.action}
                  position={POSITION}
                />
                {isSearch && showSearch && createPortal(
                  <SearchPopover onClose={() => setShowSearch(false)} />,
                  document.body
                )}
              </div>
            );
          }
          if (item.path === '/llm-chat') {
            return (
              <div ref={aiButtonRef} key={item.path} data-ai-button="true">
                <NavButton
                  path={item.path}
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  isActive={showQuickChat || location.pathname.startsWith(item.path)}
                  onClick={() => {
                    if (location.pathname.startsWith('/llm-chat')) return;
                    setShowQuickChat((prev) => !prev);
                  }}
                  position={POSITION}
                />
                {showQuickChat && createPortal(
                  <AiQuickChatPopover
                    position={quickChatPos}
                    onClose={() => setShowQuickChat(false)}
                    placement="bottom"
                  />,
                  document.body
                )}
              </div>
            );
          }
          const isActive = item.path === '/'
            ? location.pathname === '/'
            : location.pathname.startsWith(item.path);
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
        })}
      </div>
    </header>
  );
}
