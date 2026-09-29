import { type ChatMsg } from '@/pages/ai/ChatMessageBubble';

export type { ChatMsg };

export interface ConversationSummary {
  id: string;
  name: string;
  updatedAt: string;
  messageCount: number;
  deletedAt?: string;
}

export interface Conversation {
  id: string;
  name: string;
  isTemporary: boolean;
  messages: ChatMsg[];
  updatedAt: string;
  deletedAt?: string;
}

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
