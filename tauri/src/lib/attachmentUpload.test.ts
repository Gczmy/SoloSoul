import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { uploadAttachmentsSequentially, uploadSingleAttachment } from './attachmentUpload';

const sourcePath = 'C:\\Users\\Alice\\Private\\passport.pdf';

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === 'fs_get_file_size') return 128;
    if (command === 'attachment_copy_to_vault') return 'C:\\Vault\\attachment.pdf';
    return undefined;
  });
});

describe('RF-927 Windows attachment file names', () => {
  it('passes only the basename into Vault copy and metadata', async () => {
    await uploadSingleAttachment(sourcePath, 'object-a');

    expect(invoke).toHaveBeenCalledWith(
      'attachment_copy_to_vault',
      expect.objectContaining({ srcPath: sourcePath, fileName: 'passport.pdf' }),
    );
    expect(invoke).toHaveBeenCalledWith(
      'attachment_save',
      expect.objectContaining({
        meta: expect.objectContaining({
          fileName: 'passport.pdf',
          mimeType: 'application/pdf',
          srcPath: sourcePath,
        }),
      }),
    );
  });

  it('reports basenames rather than local paths in sequential upload progress', async () => {
    const onProgress = vi.fn();
    await uploadAttachmentsSequentially(
      [sourcePath, 'C:\\Users\\Alice\\Private\\notes.txt'],
      'object-a',
      onProgress,
    );

    expect(onProgress.mock.calls).toEqual([
      [0, 2, 'passport.pdf'],
      [1, 2, 'notes.txt'],
    ]);
  });
});
