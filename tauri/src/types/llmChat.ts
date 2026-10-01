import type { Conversation as ConversationWire } from '@/lib/generated/ipcContracts';
import { type ChatMsg } from '@/pages/ai/ChatMessageBubble';

export type { ChatMsg };

// wire 由 Rust 生成；消息 id/isError 仅用于展示和乐观请求。
export type { ConversationSummary } from '@/lib/generated/ipcContracts';
export type Conversation = Omit<ConversationWire, 'messages'> & { messages: ChatMsg[] };

export interface ActiveProvider {
  id: string;
  name: string;
  model: string;
  baseUrl: string;
  apiType: string;
}

export function nowISO(): string {
  return new Date().toISOString();
}

export function isOllama(baseUrl: string): boolean {
  try {
    const url = new URL(baseUrl);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return false;
    const host = url.hostname.toLowerCase().replace(/\.$/, '');
    return host === 'localhost' || host === '127.0.0.1' || host === '[::1]';
  } catch {
    return false;
  }
}

export function generateId(): string {
  return 'conv_' + crypto.randomUUID();
}
