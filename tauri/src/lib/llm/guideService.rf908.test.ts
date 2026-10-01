import { afterEach, beforeEach, describe, it, expect, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { searchGuideChunks, type GuideChunk } from './guideService';
import { createSessionRequests, setRequestSession } from '@/lib/sessionRequests';
import requests from '../../../src-tauri/src/commands/llm/rf908-requests.json';

const chunks: GuideChunk[] = [
  { guideId: 'rf908-guide', guideTitle: '合成指南', chunkText: '合成内容', similarity: 0.8 },
];
function deferred() {
  let resolve!: (value: GuideChunk[]) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<GuideChunk[]>((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}
beforeEach(() => {
  vi.clearAllMocks();
  setRequestSession(requests.bound.accountId);
});
afterEach(() => {
  setRequestSession(null);
});

describe('RF908 production guide service through the native invoke boundary', () => {
  it.each([{ result: chunks }, { result: [] as GuideChunk[] }])(
    'binds the starting account and keeps the actual populated/empty result: %j',
    async ({ result }) => {
      vi.mocked(invoke).mockResolvedValue(result);
      await expect(
        searchGuideChunks(requests.bound.accountId, requests.bound.query, requests.bound.language),
      ).resolves.toEqual(result);
      expect(invoke).toHaveBeenCalledExactlyOnceWith('llm_search_guide_chunks', requests.bound);
    },
  );
  it('keeps the original empty-context fallback for a current request failure', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('synthetic unavailable guide index'));
    await expect(
      searchGuideChunks(requests.bound.accountId, requests.bound.query, requests.bound.language),
    ).resolves.toEqual([]);
    expect(invoke).toHaveBeenCalledWith('llm_search_guide_chunks', requests.bound);
  });
  it('does not send a request from a different starting account', async () => {
    await expect(searchGuideChunks('old-account', '主密码', 'zh-CN')).rejects.toThrow(
      'expired session',
    );
    expect(invoke).not.toHaveBeenCalled();
  });
  for (const nextAccount of [null, 'new-account']) {
    it.each(['resolve', 'reject'] as const)(
      `discards delayed %s after ${nextAccount === null ? 'lock' : 'account switch'}`,
      async (outcome) => {
        const pending = deferred();
        vi.mocked(invoke).mockReturnValue(pending.promise);
        const originalTicket = createSessionRequests().begin(undefined, requests.bound.accountId);
        const search = searchGuideChunks(
          requests.bound.accountId,
          requests.bound.query,
          requests.bound.language,
          originalTicket,
        );
        const assertion = expect(search).rejects.toThrow('expired session');
        expect(invoke).toHaveBeenCalledWith('llm_search_guide_chunks', requests.bound);
        setRequestSession(nextAccount);
        if (outcome === 'resolve') pending.resolve(chunks);
        else pending.reject(new Error('synthetic late failure'));
        await assertion;
        expect(invoke).toHaveBeenCalledTimes(1);
      },
    );
  }
});
