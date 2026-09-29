import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from './authStore';
import { useProfileStore } from './profileStore';

describe('profileStore IPC contract', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'Account A' });
    useProfileStore.getState().clear();
  });

  it('keeps the requested account identity when loading a Rust Profile', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      id: 'acc-a',
      name: 'Account A',
      data: Array.from(
        new TextEncoder().encode(
          JSON.stringify({
            sections: [
              {
                type: 'identity',
                fields: [
                  { key: 'name', label: 'Name', value: 'Alice', sensitivityLevel: 'public' },
                ],
              },
            ],
          }),
        ),
      ),
      created_at: '2026-09-29T00:00:00Z',
      updated_at: '2026-09-29T00:00:00Z',
      version: 1,
    });

    await useProfileStore.getState().loadProfile('acc-a');

    expect(invoke).toHaveBeenCalledWith('profile_load', { accountId: 'acc-a' });
    expect(useProfileStore.getState()).toMatchObject({
      accountId: 'acc-a',
      isLoading: false,
      error: null,
      sections: [
        {
          sectionType: 'identity',
          fields: [{ key: 'name', label: 'Name', value: 'Alice', sensitivityLevel: 'public' }],
        },
      ],
    });
  });

  it('rejects a profile returned for another account', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      id: 'acc-b',
      data: Array.from(new TextEncoder().encode(JSON.stringify({ sections: [] }))),
    });

    await useProfileStore.getState().loadProfile('acc-a');

    expect(useProfileStore.getState()).toMatchObject({
      accountId: null,
      sections: [],
      isLoading: false,
      error: 'Error: Profile account mismatch',
    });
  });
});
