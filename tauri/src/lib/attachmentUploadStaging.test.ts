import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { cleanupStagedFile, stageFileForUpload } from './mobileFileTransfer';
import { uploadSingleAttachment } from './attachmentUpload';

vi.mock('./mobileFileTransfer', () => ({
  isUriPath: (path: string) => path.startsWith('file://'),
  stageFileForUpload: vi.fn(),
  cleanupStagedFile: vi.fn(),
}));

const sourceUri = 'file:///Downloads/notes.txt';
const stagedPath = '/cache/staged-notes.txt';

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(stageFileForUpload).mockReset().mockResolvedValue({ localPath: stagedPath, size: 42 });
  vi.mocked(cleanupStagedFile).mockReset().mockResolvedValue(undefined);
});

describe('RF-928 attachment upload staging cleanup', () => {
  it('removes the staged copy when the Vault copy fails', async () => {
    const copyError = new Error('Vault copy failed');
    vi.mocked(invoke).mockRejectedValueOnce(copyError);

    await expect(uploadSingleAttachment(sourceUri, 'object-a')).rejects.toThrow(copyError);
    expect(stageFileForUpload).toHaveBeenCalledExactlyOnceWith(sourceUri);
    expect(cleanupStagedFile).toHaveBeenCalledExactlyOnceWith(stagedPath);
    expect(invoke).not.toHaveBeenCalledWith('attachment_save', expect.anything());
  });

  it('still removes the staged copy after a successful Vault copy', async () => {
    vi.mocked(invoke).mockResolvedValueOnce('/vault/notes.txt').mockResolvedValueOnce(undefined);

    await uploadSingleAttachment(sourceUri, 'object-a');
    expect(cleanupStagedFile).toHaveBeenCalledExactlyOnceWith(stagedPath);
    expect(invoke).toHaveBeenCalledWith('attachment_save', expect.anything());
  });
});
