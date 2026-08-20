import i18n from '@/lib/i18n';
import {
  buildSystemPrompt,
  buildMessagesWithSystemPromptAndGuide,
} from '@/lib/llm/systemPromptBuilder';
import { searchGuideChunks, formatChunksAsSystemMessage } from '@/lib/llm/guideService';
import type { ChatMsg } from '@/types/llmChat';

export interface BuildChatRequestMessagesOptions {
  /** 用户当前输入。 */
  text: string;
  /** 已追加用户消息的历史（含本次用户消息）。 */
  history: ChatMsg[];
  /** 是否注入系统提示词与指南上下文。 */
  includeSystemPrompt: boolean;
}

/**
 * 组装发送给 LLM 的消息数组：
 * - includeSystemPrompt：注入系统提示词 + 按输入检索的指南文档上下文；
 * - 否则仅历史消息 + 当前输入。
 */
export async function buildChatRequestMessages({
  text,
  history,
  includeSystemPrompt,
}: BuildChatRequestMessagesOptions): Promise<Array<{ role: string; content: string }>> {
  if (!includeSystemPrompt) {
    return [
      ...history.map((m) => ({ role: m.role, content: m.content })),
      { role: 'user', content: text },
    ];
  }
  const systemPrompt = buildSystemPrompt();
  const chunks = await searchGuideChunks(text, i18n.language || 'zh-CN');
  const docPrompt = formatChunksAsSystemMessage(chunks);
  return buildMessagesWithSystemPromptAndGuide(text, history, systemPrompt, docPrompt);
}
