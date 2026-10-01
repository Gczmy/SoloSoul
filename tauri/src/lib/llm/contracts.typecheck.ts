/** RF-303：只由 tsc 检查，函数不会执行。 */
import type { IpcEvents, ConversationInput, Conversation } from '@/lib/generated/ipcContracts';
import { invokeTypedCommand } from '@/lib/typedIpc';
import { createSessionRequests } from '@/lib/sessionRequests';
import { invokeConversationChange, type StreamRun } from '@/stores/llmStore';

export async function assertLlmContract(run: StreamRun) {
  const accountId = 'synthetic-account',
    conversationId = 'synthetic-conversation';
  const args = { accountId, conversationId, providerId: 'synthetic-provider', messages: [] };
  const request = createSessionRequests().begin(undefined, accountId);
  const conversation: ConversationInput = {
    id: conversationId,
    name: 'Synthetic',
    isTemporary: true,
    updatedAt: '',
    deletedAt: null,
    messages: [{ role: 'legacy-tool-role', content: 'Old history', createdAt: '' }],
  };
  const stored: Conversation = await run.invokeTyped('llm_get_conversation', {
    accountId,
    conversationId,
  });
  const list = await request.invokeTyped('llm_list_conversations', { accountId });
  const trash = await request.invokeTyped('llm_list_trash', { accountId });
  const saved: null = await invokeTypedCommand('llm_save_conversation', {
    accountId,
    conversation,
  });
  const result: null = await run.invokeTyped('llm_send_message_stream', args);
  await invokeTypedCommand('llm_send_message_stream', {
    ...args,
    requestId: 'synthetic-request',
    contextSelection: { mode: 'none' },
  });
  await invokeTypedCommand('llm_send_message_stream', {
    ...args,
    contextSelection: { mode: 'publicProfile', objectIds: [], language: 'zh-CN', guideChunks: [] },
  });
  await invokeConversationChange('llm_rename_conversation', {
    accountId,
    conversationId,
    name: 'Renamed',
  });
  await invokeConversationChange('llm_soft_delete_conversation', { accountId, conversationId });
  await invokeConversationChange('llm_restore_conversation', { accountId, conversationId });
  await invokeConversationChange('llm_permanent_delete', { accountId, conversationId });
  const event: IpcEvents['llm-stream-chunk'] = {
    accountId,
    conversationId,
    requestId: 'synthetic-request',
    sessionGeneration: 1,
    chunk: 'Final text',
    isDone: true,
    error: null,
  };
  const failure: typeof event = { ...event, chunk: '', error: '__LLM_PERSIST_FAILED__: synthetic' };
  // @ts-expect-error ordinary send does not accept API credentials, including named variables.
  void run.invokeTyped('llm_send_message_stream', { ...args, apiKey: 'synthetic' });
  const withKey = { ...args, apiKey: 'synthetic' };
  // @ts-expect-error named variables cannot add credential fields either.
  void invokeTypedCommand('llm_send_message_stream', withKey);
  // @ts-expect-error accountId is required.
  void request.invokeTyped('llm_list_conversations', {});
  // @ts-expect-error conversationId is required.
  void run.invokeTyped('llm_get_conversation', { accountId });
  // @ts-expect-error renaming requires the new name.
  void invokeConversationChange('llm_rename_conversation', { accountId, conversationId });
  const wrongMutation = { accountId, conversationId, requestId: 'unrelated' };
  // @ts-expect-error metadata mutation is not a stream request.
  void invokeConversationChange('llm_soft_delete_conversation', wrongMutation);
  // @ts-expect-error restore does not take the rename argument.
  void invokeConversationChange('llm_restore_conversation', {
    accountId,
    conversationId,
    name: 'Unexpected',
  });
  // @ts-expect-error selected profile context requires guideChunks.
  void invokeTypedCommand('llm_send_message_stream', {
    ...args,
    contextSelection: { mode: 'publicProfile', objectIds: [], language: 'zh-CN' },
  });
  const incompleteMessage: ConversationInput = {
    ...conversation,
    // @ts-expect-error 保存消息需要 createdAt。
    messages: [{ role: 'user', content: 'text' }],
  };
  const { accountId: _account, ...withoutAccount } = event;
  const { conversationId: _conversation, ...withoutConversation } = event;
  const { requestId: _request, ...withoutRequest } = event;
  const { sessionGeneration: _generation, ...withoutGeneration } = event;
  const { error: _error, ...withoutError } = event;
  // @ts-expect-error event account identity is required.
  const noAccount: typeof event = withoutAccount;
  // @ts-expect-error event conversation identity is required.
  const noConversation: typeof event = withoutConversation;
  // @ts-expect-error event request identity is required.
  const noRequest: typeof event = withoutRequest;
  // @ts-expect-error event session generation is required.
  const noGeneration: typeof event = withoutGeneration;
  // @ts-expect-error nullable error still requires the serialized key.
  const noError: typeof event = withoutError;
  // @ts-expect-error events use camelCase, not backend field identifiers.
  const wrongGeneration: typeof event = { ...event, session_generation: 1 };
  // @ts-expect-error output skips absent deletedAt rather than serializing null.
  const nullableOutput: Conversation = { ...stored, deletedAt: null };
  void incompleteMessage;
  void noAccount;
  void noConversation;
  void noRequest;
  void noGeneration;
  void noError;
  void wrongGeneration;
  void nullableOutput;
  void _account;
  void _conversation;
  void _request;
  void _generation;
  void _error;
  return { stored, list, trash, saved, result, event, failure };
}
