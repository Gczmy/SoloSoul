import { useState } from 'react';
import { useNavCardPosition, type PopoverPlacement } from './navCardPosition';
import { useSettingsStore } from '@/stores/settingsStore';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import type { PageIconKey } from '@/lib/pageIcons';

interface NavLink {
  type: 'link';
  path: string;
  iconKey: PageIconKey;
  labelKey: string;
}

export interface NavAction {
  type: 'action';
  iconKey: PageIconKey;
  labelKey: string;
  action: () => void;
}

type NavItem = NavLink | NavAction;

export const SYSTEM_PAGE_KEYS = [
  'identity',
  'travel',
  'financial',
  'professional',
  'document',
] as const;

export const primaryItems: NavLink[] = [
  { type: 'link', path: '/', iconKey: 'home', labelKey: 'home' },
  { type: 'link', path: '/workspace?section=identity', iconKey: 'identity', labelKey: 'identity' },
  { type: 'link', path: '/workspace?section=travel', iconKey: 'travel', labelKey: 'travel' },
  {
    type: 'link',
    path: '/workspace?section=financial',
    iconKey: 'financial',
    labelKey: 'financial',
  },
  {
    type: 'link',
    path: '/workspace?section=professional',
    iconKey: 'professional',
    labelKey: 'professional',
  },
  {
    type: 'link',
    path: '/workspace?section=document',
    iconKey: 'document',
    labelKey: 'document',
  },
];

/** 下方功能按钮的可选 ID（侧边栏 3 个可变位置 + 固定的锁定/设置）。
 *  ID 同时也是 PAGE_ICON_MAP 的 key 和 i18n navigation 命名空间的 key。 */
export const CUSTOMIZABLE_ACTION_IDS = [
  'search',
  'trash',
  'templates',
  'attachments',
  'plugins',
  'ocr',
  'import_export',
  'sync',
  'help',
  'ai_chat',
] as const;

type CustomizableActionId = (typeof CUSTOMIZABLE_ACTION_IDS)[number];

/** 每个可变按钮的路由或动作工厂。
 *  lock / settings 永远固定，不在这里定义。
 *  search 与 ocr 是动作型按钮，不在这里定义路由。 */
export const CUSTOMIZABLE_LINKS: Record<
  Exclude<CustomizableActionId, 'search' | 'ocr' | 'plugins'>,
  { path: string; iconKey: PageIconKey; labelKey: string }
> = {
  ai_chat: { path: '/llm-chat', iconKey: 'ai_chat', labelKey: 'ai_chat' },
  trash: { path: '/settings/trash', iconKey: 'trash', labelKey: 'trash' },
  help: { path: '/help', iconKey: 'help', labelKey: 'help' },
  templates: { path: '/settings/templates', iconKey: 'templates', labelKey: 'templates' },
  attachments: { path: '/settings/attachments', iconKey: 'attachments', labelKey: 'attachments' },
  import_export: {
    path: '/settings/export-import',
    iconKey: 'import_export',
    labelKey: 'import_export',
  },
  sync: { path: '/sync', iconKey: 'sync', labelKey: 'sync' },
};

export function useActiveCustomPages() {
  const customPages = useSettingsStore((s) => s.settings.customPages);
  return customPages.filter((p) => !p.deletedAt);
}

/**
 * All customizable function-button IDs shown in the foldable sidebar area.
 * ID is also the PAGE_ICON_MAP key and i18n navigation namespace key.
 */
export const CARD_ACTION_IDS = ['ocr', 'plugins', 'ai_chat', 'search'] as const;

/** Check if a card-supporting button is in 'card' mode */
function isCardMode(sidebarButtonModes: Record<string, 'card' | 'page'>, id: string): boolean {
  return sidebarButtonModes[id] !== 'page';
}

/** Get the full-page path for a card-supporting button */
function getPagePath(id: string): string | undefined {
  const pageMap: Record<string, string> = {
    ocr: '/ocr',
    plugins: '/plugins',
    ai_chat: '/llm-chat',
    search: '/search',
  };
  return pageMap[id];
}

interface UseBoundNavActionsResult {
  items: NavItem[];
  showSearch: boolean;
  setShowSearch: (value: boolean) => void;
}

/** Build NavItems for all customizable function buttons (no lock/settings appended). */
export function useBoundNavActions(): UseBoundNavActionsResult {
  const [showSearch, setShowSearch] = useState(false);
  const sidebarButtonModes = useSettingsStore((s) => s.settings.sidebarButtonModes);

  const items: NavItem[] = CUSTOMIZABLE_ACTION_IDS.map((id) => {
    if ((CARD_ACTION_IDS as readonly string[]).includes(id)) {
      if (isCardMode(sidebarButtonModes, id)) {
        // Card mode: return NavAction (floating panel)
        if (id === 'search') {
          return {
            type: 'action',
            iconKey: 'search',
            labelKey: 'search',
            action: () => setShowSearch(true),
          } as NavAction;
        }
        if (id === 'ocr') {
          return {
            type: 'action',
            iconKey: 'ocr',
            labelKey: 'ocr',
            action: () => {
              const s = useOcrScanStore.getState();
              s.setCardOpen(!s.isCardOpen);
            },
          } as NavAction;
        }
        if (id === 'plugins') {
          return {
            type: 'action',
            iconKey: 'plugins',
            labelKey: 'plugin',
            action: () => {
              const s = usePluginQuickStore.getState();
              s.toggleOpen();
            },
          } as NavAction;
        }
        // ai_chat in card mode: still return NavLink but handle in consumer
        return {
          type: 'link',
          path: '/llm-chat',
          iconKey: 'ai_chat',
          labelKey: 'ai_chat',
        } as NavLink;
      }
      // Page mode: return NavLink to the dedicated page
      const path = getPagePath(id);
      if (path) {
        return {
          type: 'link',
          path,
          iconKey: id as 'ocr' | 'plugins' | 'ai_chat' | 'search',
          labelKey: id,
        } as NavLink;
      }
    }

    const link =
      CUSTOMIZABLE_LINKS[id as Exclude<CustomizableActionId, 'search' | 'ocr' | 'plugins'>];
    if (!link) {
      // Fallback for unknown ID
      return {
        type: 'action',
        iconKey: 'search',
        labelKey: 'search',
        action: () => setShowSearch(true),
      } as NavAction;
    }
    return { type: 'link', ...link } as NavLink;
  });

  return { items, showSearch, setShowSearch };
}

/** 移动端底部展开功能按钮区强制使用页面模式（AI 对话、插件、OCR、搜索均进入对应页面）。 */
export function useMobileNavActions(): UseBoundNavActionsResult {
  const { items, showSearch, setShowSearch } = useBoundNavActions();
  const mobileItems = items.map((item): NavItem => {
    if (item.type === 'link') return item;
    const id = item.iconKey;
    const path = getPagePath(id);
    if (path) {
      return { type: 'link', path, iconKey: id as PageIconKey, labelKey: id } as NavLink;
    }
    const link =
      CUSTOMIZABLE_LINKS[id as Exclude<CustomizableActionId, 'search' | 'ocr' | 'plugins'>];
    if (link) {
      return { type: 'link', ...link } as NavLink;
    }
    return item;
  });
  return { items: mobileItems, showSearch, setShowSearch };
}

/** AI 开关与定位共用同一状态，侧栏和横向导航无需再维护第二套监听。 */
export function useAiQuickChat(cardHeight = 520, placement: PopoverPlacement = 'left') {
  const [showQuickChat, setShowQuickChat] = useState(false);
  const { buttonRef, position } = useNavCardPosition(showQuickChat, cardHeight, placement);
  return { showQuickChat, setShowQuickChat, aiButtonRef: buttonRef, quickChatPos: position };
}

export function useOcrQuickScan(cardHeight = 560, placement: PopoverPlacement = 'left') {
  const isCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const { buttonRef, position } = useNavCardPosition(isCardOpen, cardHeight, placement);
  return { ocrButtonRef: buttonRef, quickScanPos: position };
}

export function usePluginQuickPanel(cardHeight = 560, placement: PopoverPlacement = 'left') {
  const isOpen = usePluginQuickStore((s) => s.isOpen);
  const { buttonRef, position } = useNavCardPosition(isOpen, cardHeight, placement);
  return { pluginButtonRef: buttonRef, quickPanelPos: position };
}
