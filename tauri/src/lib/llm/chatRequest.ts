import i18n from '@/lib/i18n';
import { useObjectStore } from '@/stores/objectStore';
import { searchGuideChunks, type GuideChunk } from '@/lib/llm/guideService';
import type { ChatMsg } from '@/types/llmChat';

/** 仅出站请求限制角色；持久化历史仍允许读取旧版本保存的其他角色。 */
export interface ChatRequestMessage {
  role: 'user' | 'assistant';
  content: string;
}

export type ChatContextSelection =
  | { mode: 'none' }
  | {
      mode: 'publicProfile';
      objectIds: string[];
      language: string;
      guideChunks: GuideChunk[];
    };

export interface ChatRequest {
  messages: ChatRequestMessage[];
  contextSelection: ChatContextSelection;
}

export interface BuildChatRequestOptions {
  /** 用户当前输入；由 builder 追加一次。 */
  text: string;
  /** 本次输入之前的原始历史；不修改其中的内容或角色。 */
  history: ChatMsg[];
  /** 是否请求 Host 构造系统上下文。 */
  includeSystemPrompt: boolean;
}

/** 前端只传消息与选择意图，系统提示词和 Vault 字段内容均由 Host 构建。 */
export async function buildChatRequest({
  text,
  history,
  includeSystemPrompt,
}: BuildChatRequestOptions): Promise<ChatRequest> {
  const messages = history.flatMap<ChatRequestMessage>((message) =>
    message.role === 'user' || message.role === 'assistant'
      ? [{ role: message.role, content: message.content }]
      : [],
  );
  messages.push({ role: 'user', content: text });

  if (!includeSystemPrompt) return { messages, contextSelection: { mode: 'none' } };

  const language = i18n.language || 'zh-CN';
  const objectIds = useObjectStore
    .getState()
    .objects.filter((object) => object.sensitivityLevel === 'public' && !object.isDeleted)
    .slice(0, 3)
    .map((object) => object.id);
  const guideChunks = await searchGuideChunks(text, language);
  return {
    messages,
    contextSelection: { mode: 'publicProfile', objectIds, language, guideChunks },
  };
}
