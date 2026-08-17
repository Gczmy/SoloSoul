// 导航按钮卡片渲染共享 hook（P142）。
// TopFunctionBar 与 SecondaryActionBar 的 renderButtonWithCard（plugins/ocr/search/
// ai_chat 四类卡片按钮 + 弹层 portal）与 renderPlainButton 此前逐字重复，此处收敛。
// 差异点经参数注入：position、quick-chat setter、各按钮 ref 与弹层位置、placement 方向。

import { createPortal } from 'react-dom';
import { lazy, Suspense } from 'react';
import { useTranslation } from 'react-i18next';
import type { NavigateFunction, Location } from 'react-router-dom';
import type { Dispatch, MutableRefObject, ReactNode, SetStateAction } from 'react';
import { NavButton } from './NavButton';
import { SearchPopover } from './SearchPopover';
import { OcrQuickScanPopover } from './OcrQuickScanPopover';
import { PluginQuickPanel } from '@/components/plugin/PluginQuickPanel';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { PageIconKey } from '@/lib/pageIcons';
import type { NavPosition } from './NavButton';

// P015-R2: AiQuickChatPopover（内部 ChatMessageList→SafeMarkdown 携带 markdown 栈）
// 由共享路径静态导入改为懒加载——弹层仅打开时拉取，markdown 栈不再进入
// 首页/页面共享 chunk 的静态依赖图（PageContainer 561K 拆分目标之一）。
const AiQuickChatPopover = lazy(() =>
  import('./AiQuickChatPopover').then((m) => ({ default: m.AiQuickChatPopover })),
);

/** 与 useNavigationItems 的 NavItem 兼容的共享形状（该类型为模块私有）。 */
export interface SharedNavLink {
  type: 'link';
  iconKey: PageIconKey;
  labelKey: string;
  path: string;
}
export interface SharedNavAction {
  type: 'action';
  iconKey: PageIconKey;
  labelKey: string;
  action: () => void;
}
export type SharedNavItem = SharedNavLink | SharedNavAction;

/** 弹层放置方向（与 AiQuickChatPopover/OcrQuickScanPopover/PluginQuickPanel 兼容）。 */
export type PopoverPlacement = 'top' | 'right' | 'bottom' | 'left';

export interface NavButtonCardInput {
  position: NavPosition;
  navigate: NavigateFunction;
  location: Location;
  showSearch: boolean;
  setShowSearch: (v: boolean) => void;
  showQuickChat: boolean;
  setShowQuickChat: Dispatch<SetStateAction<boolean>>;
  pluginButtonRef: MutableRefObject<HTMLDivElement | null>;
  ocrButtonRef: MutableRefObject<HTMLDivElement | null>;
  aiButtonRef: MutableRefObject<HTMLDivElement | null>;
  quickChatPos: { top: number } | null;
  quickScanPos: { top: number } | null;
  quickPanelPos: { top: number } | null;
  placements: {
    quickChat: PopoverPlacement;
    quickScan: PopoverPlacement;
    pluginPanel: PopoverPlacement;
  };
}

/** 提供导航卡片/普通按钮渲染逻辑（消除两处导航栏的逐字重复）。 */
export function useNavButtonCards(input: NavButtonCardInput) {
  const { t } = useTranslation('navigation');
  const {
    position,
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
    placements,
  } = input;
  const isPluginPanelOpen = usePluginQuickStore((s) => s.isOpen);
  const isOcrCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const aiChatMode = useSettingsStore((s) => s.settings.sidebarButtonModes['ai_chat']);

  /** 渲染带卡片弹层的功能按钮；非卡片项转发给 renderPlainButton。 */
  const renderButtonWithCard = (item: SharedNavItem): ReactNode => {
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
            onClick={item.type === 'action' ? item.action : () => {}}
            position={position}
          />
          {isPluginPanelOpen &&
            createPortal(
              <PluginQuickPanel
                position={quickPanelPos}
                onClose={() => usePluginQuickStore.getState().setOpen(false)}
                placement={placements.pluginPanel}
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
            onClick={item.type === 'action' ? item.action : () => {}}
            position={position}
          />
          {isOcrCardOpen &&
            createPortal(
              <OcrQuickScanPopover
                position={quickScanPos}
                onClose={() => useOcrScanStore.getState().setCardOpen(false)}
                placement={placements.quickScan}
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
            onClick={item.type === 'action' ? item.action : () => {}}
            position={position}
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
            // P015-R5: 仅「页面模式」下悬停预取 /llm-chat；卡片模式点击是打开弹层而非导航。
            // 用 prefetchPath 而非 path，避免页面模式下多出 active 指示点（未请求的视觉变化）。
            prefetchPath={aiChatMode === 'page' ? '/llm-chat' : undefined}
            position={position}
          />
          {showQuickChat &&
            createPortal(
              <Suspense
                fallback={
                  // P015-R4: chunk 拉取期占位（对齐 AiQuickChatPopover 卡片几何，
                  // 避免弹层打开瞬间空白）。位置/尺寸与 styles.card 保持一致。
                  <div
                    data-testid="quick-chat-loading"
                    style={{
                      position: 'fixed',
                      zIndex: 200,
                      width: 380,
                      height: 520,
                      maxWidth: 'calc(100vw - 24px)',
                      maxHeight: 'calc(100vh - 24px)',
                      background: 'var(--bg-elevated)',
                      borderRadius: 14,
                      boxShadow: 'var(--shadow-lg), 0 0 0 1px var(--border-subtle)',
                      border: '1px solid var(--border-subtle)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      ...(placements.quickChat === 'right'
                        ? { right: 52, left: 'auto' }
                        : placements.quickChat === 'top' || placements.quickChat === 'bottom'
                          ? { right: 12, left: 'auto' }
                          : { left: 52, right: 'auto' }),
                      top: quickChatPos?.top ?? 100,
                    }}
                  >
                    <div
                      className="spinner"
                      style={{ width: 24, height: 24, borderTopColor: 'var(--text-secondary)' }}
                    />
                  </div>
                }
              >
                <AiQuickChatPopover
                  position={quickChatPos}
                  onClose={() => setShowQuickChat(false)}
                  placement={placements.quickChat}
                />
              </Suspense>,
              document.body,
            )}
        </div>
      );
    }
    return null;
  };

  /** 渲染普通导航按钮（非卡片模式）。 */
  const renderPlainButton = (item: SharedNavItem): ReactNode => {
    if (item.type === 'action') {
      return (
        <NavButton
          key={item.iconKey}
          Icon={PAGE_ICON_MAP[item.iconKey]}
          label={t(item.labelKey)}
          onClick={item.action}
          position={position}
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
        position={position}
      />
    );
  };

  return { renderButtonWithCard, renderPlainButton };
}
