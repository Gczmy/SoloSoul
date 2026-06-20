import { useState, useRef, useCallback, useEffect } from 'react';
import type { RefObject } from 'react';
import { useVaultStore } from '@/stores/vaultStore';
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
  'plugins',
  'ai_chat',
  'search',
  'trash',
  'help',
  'templates',
  'import_export',
  'ocr',
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

interface UseBoundNavActionsResult {
  items: NavItem[];
  showSearch: boolean;
  setShowSearch: (value: boolean) => void;
}

export function useBoundNavActions(): UseBoundNavActionsResult {
  const vaultLock = useVaultStore((s) => s.lock);
  const [showSearch, setShowSearch] = useState(false);
  const sidebarBottomActions = useSettingsStore((s) => s.settings.sidebarBottomActions);
  const sidebarButtonModes = useSettingsStore((s) => s.settings.sidebarButtonModes);

  const CARD_ACTION_IDS = ['ocr', 'plugins', 'ai_chat', 'search'] as const;

  /** Check if a card-supporting button is in 'card' mode */
  function isCardMode(id: string): boolean {
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

  const items: NavItem[] = sidebarBottomActions.map((id) => {
    // Check if this button supports card/page toggle
    if ((CARD_ACTION_IDS as readonly string[]).includes(id)) {
      if (isCardMode(id)) {
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
        // ai_chat in card mode: still return NavLink but handle in SecondaryActionBar
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

    const link = CUSTOMIZABLE_LINKS[id as Exclude<CustomizableActionId, 'search' | 'ocr' | 'plugins'>];
    if (!link) {
      // 未知 ID 回退为搜索，避免渲染错误
      return {
        type: 'action',
        iconKey: 'search',
        labelKey: 'search',
        action: () => setShowSearch(true),
      } as NavAction;
    }
    return { type: 'link', ...link } as NavLink;
  });

  // 锁定按钮永远倒数第二
  items.push({
    type: 'action',
    iconKey: 'lock',
    labelKey: 'lock_vault',
    action: vaultLock,
  } as NavAction);

  // 设置按钮永远在最底部
  items.push({
    type: 'link',
    path: '/settings',
    iconKey: 'settings',
    labelKey: 'settings',
  } as NavLink);

  return { items, showSearch, setShowSearch };
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
        top = Math.min(
          Math.max(rect.top + rect.height / 2 - cardHeight / 2, 8),
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
        top = Math.min(
          Math.max(rect.top + rect.height / 2 - cardHeight / 2, 8),
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
        top = Math.min(
          Math.max(rect.top + rect.height / 2 - cardHeight / 2, 8),
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
