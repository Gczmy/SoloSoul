import { useState, useRef, useCallback, useEffect } from 'react';
import type { RefObject } from 'react';
import { useSettingsStore } from '@/stores/settingsStore';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import type { PageIconKey } from '@/lib/pageIcons';

export interface NavLink {
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

export type NavItem = NavLink | NavAction;

export const SYSTEM_PAGE_KEYS = ['identity', 'travel', 'financial', 'professional'] as const;

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
  'help',
  'ai_chat',
] as const;

export type CustomizableActionId = (typeof CUSTOMIZABLE_ACTION_IDS)[number];

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
};

export function useActiveCustomPages() {
  const customPages = useSettingsStore((s) => s.settings.customPages);
  return customPages.filter((p) => !p.deletedAt);
}

export const LOCK_ITEM: NavAction = {
  type: 'action',
  iconKey: 'lock',
  labelKey: 'lock_vault',
  action: () => {},
};

export const SETTINGS_ITEM: NavLink = {
  type: 'link',
  path: '/settings',
  iconKey: 'settings',
  labelKey: 'settings',
};

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
    ocr: '/settings/ocr',
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

export type AiQuickChatPlacement = 'left' | 'right' | 'bottom' | 'top';

interface UseAiQuickChatResult {
  showQuickChat: boolean;
  setShowQuickChat: React.Dispatch<React.SetStateAction<boolean>>;
  aiButtonRef: RefObject<HTMLDivElement | null>;
  quickChatPos: { top: number } | null;
  updateQuickChatPos: () => void;
}

export function useAiQuickChat(
  cardHeight = 520,
  placement: AiQuickChatPlacement = 'left',
): UseAiQuickChatResult {
  const [showQuickChat, setShowQuickChat] = useState(false);
  const aiButtonRef = useRef<HTMLDivElement>(null);
  const [quickChatPos, setQuickChatPos] = useState<{ top: number } | null>(null);

  const updateQuickChatPos = useCallback(() => {
    if (aiButtonRef.current) {
      const rect = aiButtonRef.current.getBoundingClientRect();
      let top: number;
      if (placement === 'bottom') {
        top = rect.bottom + 8;
      } else if (placement === 'top') {
        top = Math.max(rect.top - cardHeight - 8, 8);
      } else {
        // left or right placement — align card top with button top
        top = Math.min(
          Math.max(rect.top, 8),
          window.innerHeight - cardHeight - 8,
        );
      }
      setQuickChatPos({ top });
    }
  }, [cardHeight, placement]);

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

  return { showQuickChat, setShowQuickChat, aiButtonRef, quickChatPos, updateQuickChatPos };
}

export type OcrQuickScanPlacement = 'left' | 'right' | 'bottom' | 'top';

interface UseOcrQuickScanResult {
  ocrButtonRef: RefObject<HTMLDivElement | null>;
  quickScanPos: { top: number } | null;
  updateQuickScanPos: () => void;
}

export function useOcrQuickScan(
  cardHeight = 560,
  placement: OcrQuickScanPlacement = 'left',
): UseOcrQuickScanResult {
  const isCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const ocrButtonRef = useRef<HTMLDivElement>(null);
  const [quickScanPos, setQuickScanPos] = useState<{ top: number } | null>(null);

  const updateQuickScanPos = useCallback(() => {
    if (ocrButtonRef.current) {
      const rect = ocrButtonRef.current.getBoundingClientRect();
      let top: number;
      if (placement === 'bottom') {
        top = rect.bottom + 8;
      } else if (placement === 'top') {
        top = Math.max(rect.top - cardHeight - 8, 8);
      } else {
        // left or right placement — align card top with button top
        top = Math.min(
          Math.max(rect.top, 8),
          window.innerHeight - cardHeight - 8,
        );
      }
      setQuickScanPos({ top });
    }
  }, [cardHeight, placement]);

  useEffect(() => {
    if (!isCardOpen) return;
    updateQuickScanPos();
    window.addEventListener('scroll', updateQuickScanPos, true);
    window.addEventListener('resize', updateQuickScanPos);
    return () => {
      window.removeEventListener('scroll', updateQuickScanPos, true);
      window.removeEventListener('resize', updateQuickScanPos);
    };
  }, [isCardOpen, updateQuickScanPos]);

  return { ocrButtonRef, quickScanPos, updateQuickScanPos };
}

export type PluginQuickPanelPlacement = 'left' | 'right' | 'bottom' | 'top';

interface UsePluginQuickPanelResult {
  pluginButtonRef: RefObject<HTMLDivElement | null>;
  quickPanelPos: { top: number } | null;
  updateQuickPanelPos: () => void;
}

export function usePluginQuickPanel(
  cardHeight = 560,
  placement: PluginQuickPanelPlacement = 'left',
): UsePluginQuickPanelResult {
  const isOpen = usePluginQuickStore((s) => s.isOpen);
  const pluginButtonRef = useRef<HTMLDivElement>(null);
  const [quickPanelPos, setQuickPanelPos] = useState<{ top: number } | null>(null);

  const updateQuickPanelPos = useCallback(() => {
    if (pluginButtonRef.current) {
      const rect = pluginButtonRef.current.getBoundingClientRect();
      let top: number;
      if (placement === 'bottom') {
        top = rect.bottom + 8;
      } else if (placement === 'top') {
        top = Math.max(rect.top - cardHeight - 8, 8);
      } else {
        // left or right placement — align card top with button top
        top = Math.min(
          Math.max(rect.top, 8),
          window.innerHeight - cardHeight - 8,
        );
      }
      setQuickPanelPos({ top });
    }
  }, [cardHeight, placement]);

  useEffect(() => {
    if (!isOpen) return;
    updateQuickPanelPos();
    window.addEventListener('scroll', updateQuickPanelPos, true);
    window.addEventListener('resize', updateQuickPanelPos);
    return () => {
      window.removeEventListener('scroll', updateQuickPanelPos, true);
      window.removeEventListener('resize', updateQuickPanelPos);
    };
  }, [isOpen, updateQuickPanelPos]);

  return { pluginButtonRef, quickPanelPos, updateQuickPanelPos };
}
