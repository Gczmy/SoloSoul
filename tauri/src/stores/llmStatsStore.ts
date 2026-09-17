import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
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

const requests = createSessionRequests();

export const useLlmStatsStore = create<LlmStatsState>((set) => ({
  stats: null,
  loading: false,
  error: null,

  loadStats: async (accountId: string) => {
    const request = requests.begin('stats', accountId);
    const setCurrent = request.guardSet<LlmStatsState>(set);
    setCurrent({ loading: true, error: null });
    try {
      request.assertCurrent();
      const stats = await llmGetStats(accountId);
      request.assertCurrent();
      setCurrent({ stats, loading: false });
    } catch (e) {
      if (!request.isCurrent()) return;
      const msg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      setCurrent({ error: msg, loading: false });
    }
  },

  resetStats: async (accountId: string) => {
    const request = requests.begin('stats', accountId);
    const setCurrent = request.guardSet<LlmStatsState>(set);
    try {
      request.assertCurrent();
      await llmResetStats(accountId);
      request.assertCurrent();
      setCurrent({ stats: null, loading: false, error: null });
    } catch (e) {
      if (!request.isCurrent()) return;
      const msg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      setCurrent({ error: msg, loading: false });
    }
  },

  clear: () => {
    requests.invalidate();
    return set({ stats: null, loading: false, error: null });
  },
}));

onRequestSessionChange(() => useLlmStatsStore.getState().clear());
