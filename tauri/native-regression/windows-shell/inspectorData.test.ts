import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { invokeCommand } from './bridge';
import {
  publicAttachments,
  publicAttachmentText,
  publicHistory,
  publicObjectData,
  publicSnapshot,
} from './inspectorData';
import { useWorkspacePasswordGuard } from './verification';

describe('isolated inspector acceptance fixture', () => {
  it('provides scrollable, distinct data and an empty object', () => {
    expect(publicHistory('public-object-0')).toHaveLength(48);
    expect(publicAttachments('public-object-0')).toHaveLength(44);
    expect(publicHistory('public-object-1')).toHaveLength(17);
    expect(publicAttachments('public-object-1')).toHaveLength(13);
    expect(publicHistory('public-object-2')).toEqual([]);
    expect(publicAttachments('public-object-2')).toEqual([]);
    for (const id of ['public-object-0', 'public-object-1']) {
      for (const entry of publicHistory(id))
        expect(publicSnapshot(entry.id)?.properties.reference).toBe(id);
      for (const file of publicAttachments(id)) {
        expect(file.objectId).toBe(id);
        expect(publicAttachmentText(file.vaultPath!)).toContain(`对象：${id}`);
      }
    }
  });
  it('does not resolve arbitrary object, snapshot or filesystem paths', () => {
    expect(publicObjectData('external-object')).toBeNull();
    expect(publicHistory('external-object')).toEqual([]);
    expect(publicSnapshot('public-object-0:snapshot:999')).toBeNull();
    expect(publicSnapshot('external-object:snapshot:1')).toBeNull();
    for (const path of [
      'D:/private/vault.txt',
      'file:///vault.txt',
      'public-fixture://public-object-0/999.txt',
    ])
      expect(publicAttachmentText(path)).toBeNull();
  });
  it('serves public metadata and virtual text without invoking native business or filesystem commands', async () => {
    vi.mocked(invoke).mockClear();
    await expect(
      invokeCommand('object_get', { objectId: 'public-object-0', accountId: 'other-account' }),
    ).resolves.toBeNull();
    const files = await invokeCommand<ReturnType<typeof publicAttachments>>('attachment_list', {
      objectId: 'public-object-1',
      showDeleted: false,
    });
    await expect(
      invokeCommand('snapshot_list', { objectId: 'public-object-1' }),
    ).resolves.toHaveLength(17);
    await expect(
      invokeCommand('fs_read_file_as_text', { path: files[0].vaultPath }),
    ).resolves.toContain('public-object-1');
    await expect(
      invokeCommand('fs_read_file_as_text', { path: 'D:/private/vault.txt' }),
    ).rejects.toThrow('Only generated public fixture text');
    expect(invoke).not.toHaveBeenCalled();
  });
  it('cannot authenticate or authorize critical field access', async () => {
    const guard = useWorkspacePasswordGuard();
    await expect(guard.passwordVerify()).resolves.toEqual({ ok: false, method: 'password' });
    await expect(guard.handlePwDialogVerify()).resolves.toBe(false);
    await expect(guard.handleBiometricUnlock()).resolves.toBe(false);
  });
});
