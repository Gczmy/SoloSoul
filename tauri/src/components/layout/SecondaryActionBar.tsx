import { createPortal } from 'react-dom';
import { useLocation, useNavigate }  from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { NavButton } from './NavButton';
import { SearchPopover } from './SearchPopover';
import { AiQuickChatPopover }  from './AiQuickChatPopover';
import { OcrQuickScanPopover }  from './OcrQuickScanPopover';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { useBoundNavActions, useAiQuickChat, useOcrQuickScan } from './useNavigationItems';
import type { NavPosition } from './NavButton';

interface SecondaryActionBarProps {
  sidebarPosition: NavPosition;
  isHorizontal: boolean;
}

export function SecondaryActionBar({ sidebarPosition, isHorizontal }: SecondaryActionBarProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('navigation');

  const { items, showSearch, setShowSearch } = useBoundNavActions();
  const ocrQuickScanPlacement = sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const { ocrButtonRef, quickScanPos } = useOcrQuickScan(560, ocrQuickScanPlacement);
  const aiQuickChatPlacement = sidebarPosition === 'bottom' ? 'top' : sidebarPosition === 'right' ? 'right' : 'left';
  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const { showQuickChat, setShowQuickChat, aiButtonRef, quickChatPos } = useAiQuickChat(520, aiQuickChatPlacement);

  const zoneStyle: React.CSSProperties = isHorizontal
    ? { flexDirection: 'row', width: 'auto', height: '100%', overflow: 'visible' }
    : { flexDirection: 'column', width: '100%', height: 'auto', overflow: 'visible' };

  return (
    <div
      className="nav-secondary"
      style={{ ...zoneStyle, flexShrink: 1, marginTop: isHorizontal ? 0 : 4 }}
    >
      {items.map((item, i) => {
        if (item.type === 'action') {
          if (item.iconKey === 'ocr') {
            return (
              <div ref={ocrButtonRef} key="ocr" data-ocr-button="true">
                <NavButton
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  isActive={isOcrCardOpen}
                  onClick={item.action}
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
          const isSearch = item.iconKey === 'search';
          return (
            <div key={`action-${i}`} style={{ position: 'relative' }}>
              <NavButton
                Icon={PAGE_ICON_MAP[item.iconKey]}
                label={t(item.labelKey)}
                isActive={isSearch && showSearch}
                onClick={item.action}
                position={sidebarPosition}
              />
              {isSearch &&
                showSearch &&
                createPortal(
                  <SearchPopover onClose={() => setShowSearch(false)} />,
                  document.body,
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
      })}
    </div>
  );
}
