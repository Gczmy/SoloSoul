import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { TFunction } from 'i18next';
import { invokeCommand } from '@/lib/ipcClient';
import { createSessionRequests, setRequestSession } from '@/lib/sessionRequests';
import type { Conversation } from '@/types/llmChat';
import { saveConversationSafely } from './conversationPersistence';

const mocks = vi.hoisted(() => ({
  showToast: vi.fn(),
  warn: vi.fn(),
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));
vi.mock('@/stores/uiStore', () => ({
  useUiStore: { getState: () => ({ showToast: mocks.showToast }) },
}));
vi.mock('@/lib/logger', () => ({ logger: { warn: mocks.warn } }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

const accountId = 'account-a';
const conversation = (): Conversation => ({
  id: 'conversation-a',
  name: 'Synthetic conversation',
  isTemporary: false,
  messages: [
    {
      id: 'user-a',
      role: 'user',
      content: 'Synthetic question',
      createdAt: '2026-09-30T00:00:00Z',
    },
    {
      id: 'assistant-a',
      role: 'assistant',
      content: 'Previously saved answer',
      createdAt: '2026-09-30T00:00:01Z',
    },
  ],
  updatedAt: '2026-09-30T00:00:02Z',
});

describe('saveConversationSafely request lifetime', () => {
  const translate = vi.fn(() => '对话保存失败，记录可能丢失，请重试');
  const t = translate as unknown as TFunction;

  beforeEach(() => {
    setRequestSession(null);
    vi.resetAllMocks();
    translate.mockReturnValue('对话保存失败，记录可能丢失，请重试');
    vi.mocked(invokeCommand).mockResolvedValue(undefined);
    setRequestSession(accountId);
  });

  afterEach(() => {
    setRequestSession(null);
  });

  it('does not dispatch, log, or notify an old ticket after locking and unlocking the same account', async () => {
    const ticket = createSessionRequests().begin(undefined, accountId);
    setRequestSession(null);
    setRequestSession(accountId);

    expect(ticket.isCurrent()).toBe(false);
    await expect(saveConversationSafely(accountId, conversation(), t, ticket)).resolves.toBe(false);
    expect(invokeCommand).not.toHaveBeenCalled();
    expect(mocks.warn).not.toHaveBeenCalled();
    expect(mocks.showToast).not.toHaveBeenCalled();
    expect(translate).not.toHaveBeenCalled();
  });

  it('contains a pending save rejection after lock without logging or showing a stale failure', async () => {
    const operation = deferred<void>();
    vi.mocked(invokeCommand).mockReturnValue(operation.promise);
    const ticket = createSessionRequests().begin(undefined, accountId);
    const saved = saveConversationSafely(accountId, conversation(), t, ticket);
    expect(invokeCommand).toHaveBeenCalledTimes(1);

    setRequestSession(null);
    operation.reject(new Error('Old database write failed'));
    await expect(saved).resolves.toBe(false);
    expect(ticket.isCurrent()).toBe(false);
    expect(mocks.warn).not.toHaveBeenCalled();
    expect(mocks.showToast).not.toHaveBeenCalled();
    expect(translate).not.toHaveBeenCalled();
  });

  it('returns false and reports one warning and one toast for a current save failure', async () => {
    const failure = new Error('Database write failed');
    vi.mocked(invokeCommand).mockRejectedValue(failure);
    const ticket = createSessionRequests().begin(undefined, accountId);

    await expect(saveConversationSafely(accountId, conversation(), t, ticket)).resolves.toBe(false);
    expect(ticket.isCurrent()).toBe(true);
    expect(invokeCommand).toHaveBeenCalledTimes(1);
    expect(mocks.warn).toHaveBeenCalledExactlyOnceWith(
      '[useLlmChatCore] Save conversation failed:',
      failure,
    );
    expect(translate).toHaveBeenCalledExactlyOnceWith('settings:ai_save_conversation_failed', {
      defaultValue: '对话保存失败，记录可能丢失，请重试',
    });
    expect(mocks.showToast).toHaveBeenCalledExactlyOnceWith({
      type: 'error',
      message: '对话保存失败，记录可能丢失，请重试',
      duration: 5000,
    });
  });

  it('saves the complete current conversation exactly once without a warning or toast', async () => {
    const value = conversation();
    const ticket = createSessionRequests().begin(undefined, accountId);

    await expect(saveConversationSafely(accountId, value, t, ticket)).resolves.toBe(true);
    expect(invokeCommand).toHaveBeenCalledExactlyOnceWith(
      'llm_save_conversation',
      { accountId, conversation: value },
      { requestIsCurrent: ticket.isCurrent },
    );
    expect(ticket.isCurrent()).toBe(true);
    expect(mocks.warn).not.toHaveBeenCalled();
    expect(mocks.showToast).not.toHaveBeenCalled();
    expect(translate).not.toHaveBeenCalled();
  });
});
