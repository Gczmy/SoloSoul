import { useState } from 'react';
import { createPortal } from 'react-dom';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Home, Settings, Lock, ChevronDown } from 'lucide-react';
import styles from './MobileBottomNav.module.css';
import { useVaultStore } from '@/stores/vaultStore';
import { AddPageButton } from './AddPageButton';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import {
  useBoundNavActions,
  useAiQuickChat,
  useOcrQuickScan,
  usePluginQuickPanel,
  CARD_ACTION_IDS,
} from './useNavigationItems';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { SearchPopover } from './SearchPopover';
import { AiQuickChatPopover } from './AiQuickChatPopover';
import { OcrQuickScanPopover } from './OcrQuickScanPopover';
import { PluginQuickPanel } from '@/components/plugin/PluginQuickPanel';
import type { NavAction } from './useNavigationItems';
import { ICON_SIZE } from '@/lib/constants';

const NAV_ITEMS = [
  { path: '/', labelKey: 'home', Icon: Home },
  { path: '/settings', labelKey: 'settings', Icon: Settings },
] as const;

export function MobileBottomNav() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('navigation');
  const lock = useVaultStore((s) => s.lock);
  const [expanded, setExpanded] = useState(false);

  const { items, showSearch, setShowSearch } = useBoundNavActions();

  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const isPluginPanelOpen = usePluginQuickStore((s) => s.isOpen);
  const aiChatMode = useSettingsStore((s) => s.settings.sidebarButtonModes['ai_chat']);

  const { showQuickChat, setShowQuickChat, aiButtonRef, quickChatPos } = useAiQuickChat(520, 'top');
  const { ocrButtonRef, quickScanPos } = useOcrQuickScan(560, 'top');
  const { pluginButtonRef, quickPanelPos } = usePluginQuickPanel(560, 'top');

  const handleNavigate = (path: string) => {
    navigate(path);
    setExpanded(false);
  };

  const handleLock = () => {
    lock();
    setExpanded(false);
  };

  const renderPlainButton = (item: (typeof items)[number]) => {
    const Icon = PAGE_ICON_MAP[item.iconKey];
    const isLink = item.type === 'link';
    const isActive = isLink
      ? item.path === '/'
        ? location.pathname === '/'
        : location.pathname.startsWith(item.path)
      : false;
    return (
      <button
        key={isLink ? item.path : item.iconKey}
        type="button"
        className={`${styles.functionButton} ${isActive ? styles.functionButtonActive : ''}`}
        onClick={() =>
          isLink ? handleNavigate(item.path) : (item as NavAction).action()
        }
      >
        <Icon size={ICON_SIZE.xl} />
        <span className={styles.functionLabel}>{t(item.labelKey)}</span>
      </button>
    );
  };

  const renderCardButton = (item: (typeof items)[number]) => {
    const iconKey = item.iconKey;

    if (iconKey === 'plugins') {
      const Icon = PAGE_ICON_MAP.plugins;
      return (
        <div ref={pluginButtonRef} key="plugins" className={styles.functionButtonWrapper}>
          <button
            type="button"
            className={`${styles.functionButton} ${isPluginPanelOpen ? styles.functionButtonActive : ''}`}
            onClick={() => {
              const s = usePluginQuickStore.getState();
              s.setOpen(!s.isOpen);
            }}
          >
            <Icon size={ICON_SIZE.xl} />
            <span className={styles.functionLabel}>{t(item.labelKey)}</span>
          </button>
          {isPluginPanelOpen &&
            createPortal(
              <PluginQuickPanel
                position={quickPanelPos}
                onClose={() => usePluginQuickStore.getState().setOpen(false)}
                placement="top"
              />,
              document.body,
            )}
        </div>
      );
    }

    if (iconKey === 'ocr') {
      const Icon = PAGE_ICON_MAP.ocr;
      return (
        <div ref={ocrButtonRef} key="ocr" className={styles.functionButtonWrapper}>
          <button
            type="button"
            className={`${styles.functionButton} ${isOcrCardOpen ? styles.functionButtonActive : ''}`}
            onClick={() => {
              const s = useOcrScanStore.getState();
              s.setCardOpen(!s.isCardOpen);
            }}
          >
            <Icon size={ICON_SIZE.xl} />
            <span className={styles.functionLabel}>{t(item.labelKey)}</span>
          </button>
          {isOcrCardOpen &&
            createPortal(
              <OcrQuickScanPopover
                position={quickScanPos}
                onClose={() => useOcrScanStore.getState().setCardOpen(false)}
                placement="top"
              />,
              document.body,
            )}
        </div>
      );
    }

    if (iconKey === 'search') {
      const Icon = PAGE_ICON_MAP.search;
      return (
        <div key="search" className={styles.functionButtonWrapper}>
          <button
            type="button"
            className={`${styles.functionButton} ${showSearch ? styles.functionButtonActive : ''}`}
            onClick={() => setShowSearch(true)}
          >
            <Icon size={ICON_SIZE.xl} />
            <span className={styles.functionLabel}>{t(item.labelKey)}</span>
          </button>
          {showSearch &&
            createPortal(<SearchPopover onClose={() => setShowSearch(false)} />, document.body)}
        </div>
      );
    }

    if (iconKey === 'ai_chat') {
      const Icon = PAGE_ICON_MAP.ai_chat;
      return (
        <div ref={aiButtonRef} key="ai_chat" className={styles.functionButtonWrapper}>
          <button
            type="button"
            className={`${styles.functionButton} ${showQuickChat || location.pathname.startsWith('/llm-chat') ? styles.functionButtonActive : ''}`}
            onClick={() => {
              if (aiChatMode === 'page') {
                handleNavigate('/llm-chat');
              } else if (!location.pathname.startsWith('/llm-chat')) {
                setShowQuickChat((prev) => !prev);
              }
            }}
          >
            <Icon size={ICON_SIZE.xl} />
            <span className={styles.functionLabel}>{t(item.labelKey)}</span>
          </button>
          {showQuickChat &&
            createPortal(
              <AiQuickChatPopover
                position={quickChatPos}
                onClose={() => setShowQuickChat(false)}
                placement="top"
              />,
              document.body,
            )}
        </div>
      );
    }

    return null;
  };

  return (
    <>
      {/* Expandable function button panel */}
      {expanded && (
        <div className={styles.functionPanel}>
          <div className={styles.functionGrid}>
            {items.map((item) => {
              const isCardButton = (CARD_ACTION_IDS as readonly string[]).includes(item.iconKey);
              if (isCardButton) {
                const cardEl = renderCardButton(item);
                if (cardEl) return cardEl;
              }
              return renderPlainButton(item);
            })}
          </div>
        </div>
      )}

      <nav className={styles.bottomNav} aria-label={t('home')}>
        {NAV_ITEMS.map(({ path, labelKey, Icon }) => {
          const isActive =
            path === '/' ? location.pathname === '/' : location.pathname.startsWith(path);
          return (
            <button
              key={path}
              type="button"
              className={`${styles.navItem} ${isActive ? styles.navItemActive : ''}`}
              onClick={() => handleNavigate(path)}
              aria-current={isActive ? 'page' : undefined}
            >
              <Icon size={22} />
              <span className={styles.navLabel}>{t(labelKey)}</span>
            </button>
          );
        })}

        {/* Add page */}
        <div className={styles.navItem}>
          <AddPageButton onCreate={(page) => handleNavigate(`/workspace/custom/${page.id}`)} position="bottom" />
        </div>

        {/* Expand / collapse toggle */}
        <button
          type="button"
          className={`${styles.navItem} ${expanded ? styles.navItemActive : ''}`}
          onClick={() => setExpanded((prev) => !prev)}
          aria-label={expanded ? t('common:collapse') : t('common:expand')}
          aria-expanded={expanded}
        >
          <ChevronDown
            size={22}
            className={`${styles.toggleIcon} ${expanded ? styles.toggleIconExpanded : ''}`}
          />
          <span className={styles.navLabel}>{expanded ? t('common:collapse') : t('common:expand')}</span>
        </button>

        {/* Lock vault quick action */}
        <button type="button" className={styles.navItem} onClick={handleLock}>
          <Lock size={22} />
          <span className={styles.navLabel}>{t('lock_vault')}</span>
        </button>
      </nav>
    </>
  );
}
