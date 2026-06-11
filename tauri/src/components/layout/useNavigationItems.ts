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
  { type: 'link', path: '/workspace?section=financial', iconKey: 'financial', labelKey: 'financial' },
  { type: 'link', path: '/workspace?section=professional', iconKey: 'professional', labelKey: 'professional' },
];

export const secondaryItems: NavItem[] = [
  { type: 'action', iconKey: 'lock', labelKey: 'lock_vault', action: () => {} },
  { type: 'action', iconKey: 'search', labelKey: 'search', action: () => {} },
  { type: 'link', path: '/plugins', iconKey: 'plugins', labelKey: 'plugin' },
  { type: 'link', path: '/llm-chat', iconKey: 'ai_chat', labelKey: 'ai_chat' },
  { type: 'link', path: '/settings', iconKey: 'settings', labelKey: 'settings' },
];

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

  const items = secondaryItems.map((item) => {
    if (item.type !== 'action') return item;
    if (item.iconKey === 'lock') {
      return { ...item, action: vaultLock } as NavAction;
    }
    if (item.iconKey === 'search') {
      return { ...item, action: () => setShowSearch(true) } as NavAction;
    }
    return item;
  });

  return { items, showSearch, setShowSearch };
}

export type AiQuickChatPlacement = 'left' | 'bottom' | 'top';

interface UseAiQuickChatResult {
  showQuickChat: boolean;
  setShowQuickChat: React.Dispatch<React.SetStateAction<boolean>>;
  aiButtonRef: RefObject<HTMLDivElement | null>;
  quickChatPos: { top: number } | null;
  updateQuickChatPos: () => void;
}

export function useAiQuickChat(
  cardHeight = 520,
  placement: AiQuickChatPlacement = 'left'
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
          window.innerHeight - cardHeight - 8
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
