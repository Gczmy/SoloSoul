import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { appCacheDir, join } from '@tauri-apps/api/path';
import { copyFile, mkdir, remove, stat } from '@tauri-apps/plugin-fs';
import { stageFileForUpload, stageImportPackage } from './mobileFileTransfer';

vi.mock('@tauri-apps/api/path', () => ({
  appCacheDir: vi.fn(),
  join: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-fs', () => ({
  copyFile: vi.fn(),
  mkdir: vi.fn(),
  remove: vi.fn(),
  stat: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(appCacheDir).mockReset().mockResolvedValue('/cache');
  vi.mocked(join)
    .mockReset()
    .mockImplementation(async (...parts) => parts.join('/'));
  vi.mocked(mkdir).mockReset().mockResolvedValue(undefined);
  vi.mocked(copyFile).mockReset().mockResolvedValue(undefined);
  vi.mocked(stat)
    .mockReset()
    .mockResolvedValue({ size: 42 } as Awaited<ReturnType<typeof stat>>);
  vi.mocked(remove).mockReset().mockResolvedValue(undefined);
});

describe('RF-929 partial mobile stage cleanup', () => {
  it('keeps completed upload and import stages available to their callers', async () => {
    const upload = await stageFileForUpload('file:///Downloads/a.txt');
    const archive = await stageImportPackage('content://provider/archive');

    expect(upload.localPath).toBe(vi.mocked(copyFile).mock.calls[0][1]);
    expect(upload.size).toBe(42);
    expect(archive).toContain('solosoul_mobile_stage');
    expect(remove).not.toHaveBeenCalled();
  });

  it('removes a copied upload stage when its size lookup fails', async () => {
    const statError = new Error('stat failed');
    vi.mocked(stat).mockRejectedValueOnce(statError);

    await expect(stageFileForUpload('file:///Downloads/a.txt')).rejects.toThrow(statError);
    expect(copyFile).toHaveBeenCalledOnce();
    const stagedPath = vi.mocked(copyFile).mock.calls[0][1];
    expect(remove).toHaveBeenCalledExactlyOnceWith(stagedPath);
  });

  it('removes a partial content URI import when native copying fails', async () => {
    const copyError = new Error('native copy failed');
    vi.mocked(invoke).mockRejectedValueOnce(copyError);

    await expect(stageImportPackage('content://provider/archive')).rejects.toThrow(copyError);
    expect(invoke).toHaveBeenCalledWith('copy_content_uri_to_path', {
      contentUri: 'content://provider/archive',
      destPath: expect.stringContaining('solosoul_mobile_stage'),
    });
    const { destPath } = vi.mocked(invoke).mock.calls[0][1] as { destPath: string };
    expect(remove).toHaveBeenCalledExactlyOnceWith(destPath);
  });
});
