import { create } from 'zustand';
import { llmGetStats, llmResetStats, type LlmUsageStats } from '@/lib/llm/statsApi';

interface LlmStatsState {
  stats: LlmUsageStats | null;
  loading: boolean;
  error: string | null;

  loadStats: (accountId: string) => Promise<void>;
  resetStats: (accountId: string) => Promise<void>;
  clear: () => void;
}

export const useLlmStatsStore = create<LlmStatsState>((set) => ({
  stats: null,
  loading: false,
  error: null,

  loadStats: async (accountId: string) => {
    set({ loading: true, error: null });
    try {
      const stats = await llmGetStats(accountId);
      set({ stats, loading: false });
    } catch (e) {
      const msg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      set({ error: msg, loading: false });
    }
  },

  resetStats: async (accountId: string) => {
    try {
      await llmResetStats(accountId);
      set({ stats: null });
    } catch (e) {
      const msg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      set({ error: msg });
    }
  },

  clear: () => set({ stats: null, loading: false, error: null }),
}));
