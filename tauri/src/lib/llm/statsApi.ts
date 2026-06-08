import { invoke } from '@tauri-apps/api/core';

export interface ModelUsage {
  model: string;
  provider: string;
  count: number;
  tokens: number;
  promptTokens: number;
  completionTokens: number;
  lastUsedTime?: string;
}

export interface DailyUsage {
  date: string; // YYYY-MM-DD
  count: number;
  tokens: number;
  perModelTokens: Record<string, number>;
}

export interface LlmUsageStats {
  usageCount: number;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  perModelStats: ModelUsage[];
  dailyStats: DailyUsage[];
}

export async function llmGetStats(accountId: string): Promise<LlmUsageStats> {
  return invoke('llm_get_stats', { accountId });
}

export async function llmResetStats(accountId: string): Promise<void> {
  return invoke('llm_reset_stats', { accountId });
}

export async function llmPersistStats(accountId: string): Promise<void> {
  return invoke('llm_persist_stats', { accountId });
}

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}
