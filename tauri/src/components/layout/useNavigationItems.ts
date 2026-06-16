import { useState, useRef, useCallback, useEffect } from 'react';
import type { RefObject } from 'react';
import { useVaultStore } from '@/stores/vaultStore';
import { useSettingsStore } from '@/stores/settingsStore';
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
 *  lock / settings 永远固定，不在这里定义。 */
export const CUSTOMIZABLE_LINKS: Record<
  Exclude<CustomizableActionId, 'search'>,
  { path: string; iconKey: PageIconKey; labelKey: string }
> = {
  plugins: { path: '/plugins', iconKey: 'plugins', labelKey: 'plugin' },
  ai_chat: { path: '/llm-chat', iconKey: 'ai_chat', labelKey: 'ai_chat' },
  trash: { path: '/settings/trash', iconKey: 'trash', labelKey: 'trash' },
  help: { path: '/help', iconKey: 'help', labelKey: 'help' },
  templates: { path: '/settings/templates', iconKey: 'templates', labelKey: 'templates' },
  import_export: {
    path: '/settings/export-import',
    iconKey: 'import_export',
    labelKey: 'import_export',
  },
  ocr: { path: '/ocr', iconKey: 'ocr', labelKey: 'ocr' },
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

  const items: NavItem[] = sidebarBottomActions.map((id) => {
    if (id === 'search') {
      return {
        type: 'action',
        iconKey: 'search',
        labelKey: 'search',
        action: () => setShowSearch(true),
      } as NavAction;
    }
    const link = CUSTOMIZABLE_LINKS[id as Exclude<CustomizableActionId, 'search'>];
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
