import { useState, useCallback, useRef, useEffect } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useLlmChatCore } from '@/hooks/useLlmChatCore';
import { useToastError } from '@/hooks/useToastError';
import { useTranslation } from 'react-i18next';
import { setAiPageOpen } from '@/lib/notification';
import {
  type ChatMsg,
  type Conversation,
  type ConversationSummary,
  nowISO,
  generateId,
} from '@/types/llmChat';
import { logger } from '@/lib/logger';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';

export type { Conversation, ConversationSummary };

export interface UseLlmChatReturn {
  activeProvider: ReturnType<typeof useLlmChatCore>['activeProvider'];
  isConfigured: boolean;
  isAiEnabled: boolean;
  includeSystemPrompt: boolean;
  loading: boolean;
  conversations: ConversationSummary[];
  trashList: ConversationSummary[];
  showTrash: boolean;
  currentConvId: string | null;
  currentConv: Conversation | null;
  messages: ChatMsg[];
  input: string;
  isSending: boolean;
  isOnline: boolean | null;
  checkingOnline: boolean;
  copiedIndex: number | null;
  floatingConv: Conversation | null;
  confirmPermanentDelete: string | null;
  isLocal: boolean;
  setInput: (v: string) => void;
  setShowTrash: (v: boolean) => void;
  setConfirmPermanentDelete: (v: string | null) => void;
  setFloatingConv: (v: Conversation | null) => void;
  sendMessage: () => Promise<void>;
  handleNewConversation: () => void;
  loadConversation: (convId: string) => Promise<void>;
  handleRename: (convId: string, newName: string) => Promise<void>;
  handleSoftDelete: (convId: string) => Promise<void>;
  handleRestore: (convId: string) => Promise<void>;
  handlePermanentDelete: (convId: string) => Promise<void>;
  handleViewTrashConv: (convId: string) => Promise<void>;
  handleCopy: (content: string, index: number) => Promise<void>;
  checkOnline: () => void;
}

export function useLlmChat(): UseLlmChatReturn {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation('common');
  const { onError } = useToastError();

  const [includeSystemPrompt] = useState(true);
  const [trashList, setTrashList] = useState<ConversationSummary[]>([]);
  const [showTrash, setShowTrash] = useState(false);
  const [currentConv, setCurrentConv] = useState<Conversation | null>(null);
  const [floatingConv, setFloatingConvState] = useState<Conversation | null>(null);
  const [confirmPermanentDelete, setConfirmPermanentDelete] = useState<string | null>(null);

  const [readRequests] = useState(createSessionRequests);
  const refreshCoreList = useRef<() => Promise<void>>(async () => {});
  const setFloatingConv = useCallback(
    (conv: Conversation | null) => {
      readRequests.invalidate('trashBody');
      setFloatingConvState(conv);
    },
    [readRequests],
  );
  useEffect(() => {
    const clear = () => {
      readRequests.invalidate();
      setTrashList([]);
      setFloatingConvState(null);
      setCurrentConv(null);
      setShowTrash(false);
      setConfirmPermanentDelete(null);
    };
    clear();
    const unsubscribe = onRequestSessionChange(clear);
    return () => {
      unsubscribe();
      readRequests.invalidate();
    };
  }, [accountId, readRequests]);
  useEffect(() => {
    setAiPageOpen(true);
    return () => setAiPageOpen(false);
  }, []);

  const loadTrashList = useCallback(async () => {
    if (!accountId) return;
    const request = readRequests.begin('trashList', accountId);
    try {
      const trash = await request.invoke<ConversationSummary[]>('llm_list_trash', { accountId });
      if (request.isCurrent()) setTrashList(trash);
    } catch (err) {
      if (request.isCurrent()) logger.warn('[useLlmChat] Refresh trash list failed:', err);
    }
  }, [accountId, readRequests]);
  const refreshLists = useCallback(() => {
    void refreshCoreList.current();
    void loadTrashList();
  }, [loadTrashList]);

  const core = useLlmChatCore({ includeSystemPrompt, onConversationSaved: refreshLists });
  refreshCoreList.current = core.loadConversationList;
  useEffect(() => {
    if (showTrash) void loadTrashList();
    else {
      readRequests.invalidate('trashBody');
      setFloatingConvState(null);
    }
  }, [showTrash, loadTrashList, readRequests]);
  // Sync core's currentConvId to our currentConv tracking
  const prevConvIdRef = useRef<string | null>(null);
  useEffect(() => {
    if (!core.currentConvId) {
      prevConvIdRef.current = null;
      setCurrentConv(null);
    }
    if (core.currentConvId && core.currentConvId !== prevConvIdRef.current) {
      prevConvIdRef.current = core.currentConvId;
      if (core.messages.length > 0) {
        setCurrentConv({
          id: core.currentConvId,
          name: '',
          isTemporary: false,
          messages: core.messages,
          updatedAt: nowISO(),
        });
      }
    }
  }, [core.currentConvId, core.messages]);

  const handleNewConversation = useCallback(() => {
    setFloatingConv(null);
    const id = generateId();
    core.setCurrentConvId(id);
    setCurrentConv({ id, name: '', isTemporary: true, messages: [], updatedAt: nowISO() });
    core.setMessages([]);
  }, [core, setFloatingConv]);

  const handleRename = useCallback(
    async (convId: string, newName: string) => {
      if (!accountId || !newName.trim()) return;
      try {
        await invoke('llm_rename_conversation', {
          accountId: accountId,
          conversationId: convId,
          name: newName.trim(),
        });
      } catch (e) {
        // P021: 失败不再静默——toast 提示且不执行后续本地更新
        logger.warn('[useLlmChat] Rename conversation failed:', e);
        onError(e, t('common:error'));
        return;
      }
      core.loadConversationList();
      if (currentConv?.id === convId)
        setCurrentConv((prev) => (prev ? { ...prev, name: newName.trim() } : prev));
    },
    [accountId, currentConv, core, onError, t],
  );

  const handleSoftDelete = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      try {
        await invoke('llm_soft_delete_conversation', {
          accountId: accountId,
          conversationId: convId,
        });
      } catch (e) {
        logger.warn('[useLlmChat] Soft delete conversation failed:', e);
        onError(e, t('common:error'));
        return;
      }
      if (core.currentConvId === convId) handleNewConversation();
      refreshLists();
    },
    [accountId, core.currentConvId, handleNewConversation, refreshLists, onError, t],
  );

  const handleRestore = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      try {
        await invoke('llm_restore_conversation', {
          accountId: accountId,
          conversationId: convId,
        });
      } catch (e) {
        logger.warn('[useLlmChat] Restore conversation failed:', e);
        onError(e, t('common:error'));
        return;
      }
      refreshLists();
    },
    [accountId, refreshLists, onError, t],
  );

  const handlePermanentDelete = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      try {
        await invoke('llm_permanent_delete', {
          accountId: accountId,
          conversationId: convId,
        });
      } catch (e) {
        logger.warn('[useLlmChat] Permanent delete conversation failed:', e);
        onError(e, t('common:error'));
        return;
      }
      setTrashList((prev) => prev.filter((c) => c.id !== convId));
      setConfirmPermanentDelete(null);
      readRequests.invalidate('trashBody');
      setFloatingConvState((prev) => (prev?.id === convId ? null : prev));
    },
    [accountId, onError, t, readRequests],
  );

  const handleViewTrashConv = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      if (floatingConv?.id === convId) {
        setFloatingConv(null);
        return;
      }
      const request = readRequests.begin('trashBody', accountId);
      setFloatingConvState(null);
      try {
        const conv = await request.invoke<Conversation>('llm_get_conversation', {
          accountId,
          conversationId: convId,
        });
        if (request.isCurrent()) setFloatingConvState(conv);
      } catch (err) {
        if (request.isCurrent()) logger.warn('[useLlmChat] Load trash conversation failed:', err);
      }
    },
    [accountId, floatingConv?.id, readRequests, setFloatingConv],
  );
  const loadConversation = useCallback(
    async (convId: string) => {
      setFloatingConv(null);
      await core.loadConversation(convId);
    },
    [core, setFloatingConv],
  );
  // Scroll to bottom on new messages
  const lastMessageKey =
    core.messages.length > 0 ? core.messages[core.messages.length - 1].createdAt : null;
  useEffect(() => {
    const el = document.querySelector('[data-chat-end]');
    el?.scrollIntoView({ behavior: 'auto' });
  }, [lastMessageKey]);

  return {
    activeProvider: core.activeProvider,
    isConfigured: core.isConfigured,
    isAiEnabled: core.isAiEnabled,
    includeSystemPrompt,
    loading: core.loading,
    conversations: core.conversations,
    trashList,
    showTrash,
    currentConvId: core.currentConvId,
    currentConv,
    messages: core.messages,
    input: core.input,
    isSending: core.isSending,
    isOnline: core.isOnline,
    checkingOnline: core.checkingOnline,
    copiedIndex: core.copiedIndex,
    floatingConv,
    confirmPermanentDelete,
    isLocal: core.isLocal,
    setInput: core.setInput,
    setShowTrash,
    setConfirmPermanentDelete,
    setFloatingConv,
    sendMessage: core.sendMessage,
    handleNewConversation,
    loadConversation,
    handleRename,
    handleSoftDelete,
    handleRestore,
    handlePermanentDelete,
    handleViewTrashConv,
    handleCopy: core.handleCopy,
    checkOnline: core.checkOnline,
  };
}
