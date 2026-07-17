import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { invoke } from '@tauri-apps/api/core';

describe('ipc interfaces', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe('Auth', () => {
    it('check_has_account 调用 invoke("check_has_account")', async () => {
      mockInvoke.mockResolvedValue(true);
      const result = (await invoke('check_has_account')) as boolean;
      expect(mockInvoke).toHaveBeenCalledWith('check_has_account');
      expect(result).toBe(true);
    });

    it('bootstrap 传递 accountName / password / locale / passwordHint', async () => {
      mockInvoke.mockResolvedValue({ id: 'acc-1', name: 'Test' });
      const result = (await invoke('bootstrap', {
        accountName: 'Test',
        password: 'pwd123',
        locale: 'zh',
        passwordHint: 'hint',
      })) as { id: string; name: string };
      expect(mockInvoke).toHaveBeenCalledWith('bootstrap', {
        accountName: 'Test',
        password: 'pwd123',
        locale: 'zh',
        passwordHint: 'hint',
      });
      expect(result.name).toBe('Test');
    });

    it('bootstrap 可不传 passwordHint', async () => {
      mockInvoke.mockResolvedValue({ id: 'acc-1', name: 'Test' });
      await invoke('bootstrap', { accountName: 'Test', password: 'pwd123', locale: 'en' });
      expect(mockInvoke).toHaveBeenCalledWith('bootstrap', {
        accountName: 'Test',
        password: 'pwd123',
        locale: 'en',
      });
    });

    it('login 传递 accountId + password', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('login', { accountId: 'acc-1', password: 'pwd123' });
      expect(mockInvoke).toHaveBeenCalledWith('login', { accountId: 'acc-1', password: 'pwd123' });
    });

    it('logout 调用 invoke("logout")', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('logout');
      expect(mockInvoke).toHaveBeenCalledWith('logout');
    });
  });

  describe('Vault', () => {
    it('unlock 传递 accountId + password', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('unlock', { accountId: 'acc-1', password: 'pwd' });
      expect(mockInvoke).toHaveBeenCalledWith('unlock', { accountId: 'acc-1', password: 'pwd' });
    });

    it('lock 调用 invoke("lock")', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('lock');
      expect(mockInvoke).toHaveBeenCalledWith('lock');
    });

    it('get_state 返回 VaultStateStr', async () => {
      mockInvoke.mockResolvedValue('unlocked');
      const result = (await invoke('get_state')) as string;
      expect(result).toBe('unlocked');
    });

    it('change_password 传递旧/新密码', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('change_password', {
        accountId: 'acc-1',
        oldPassword: 'old',
        newPassword: 'new',
      });
      expect(mockInvoke).toHaveBeenCalledWith('change_password', {
        accountId: 'acc-1',
        oldPassword: 'old',
        newPassword: 'new',
      });
    });

    it('delete_account 传递 accountId + password', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('delete_account', { accountId: 'acc-1', password: 'pwd' });
      expect(mockInvoke).toHaveBeenCalledWith('delete_account', {
        accountId: 'acc-1',
        password: 'pwd',
      });
    });

    it('vault_list_accounts 返回 AccountInfo[]', async () => {
      const accounts = [
        { id: '1', name: 'A' },
        { id: '2', name: 'B' },
      ];
      mockInvoke.mockResolvedValue(accounts);
      const result = await invoke('vault_list_accounts');
      expect(result).toHaveLength(2);
    });
  });

  describe('Crypto', () => {
    it('encrypt_bytes / decrypt_bytes 传递 data', async () => {
      mockInvoke.mockResolvedValue([1, 2, 3]);
      const enc = await invoke('encrypt_bytes', { data: [10, 20] });
      expect(mockInvoke).toHaveBeenCalledWith('encrypt_bytes', { data: [10, 20] });
      expect(enc).toEqual([1, 2, 3]);

      mockInvoke.mockResolvedValue([10, 20]);
      const dec = await invoke('decrypt_bytes', { data: [1, 2, 3] });
      expect(dec).toEqual([10, 20]);
    });

    it('deriveKey 传递所有 KDF 参数', async () => {
      mockInvoke.mockResolvedValue([1, 2, 3]);
      await invoke('derive_key', {
        password: 'pwd',
        salt: [1, 2],
        memoryKib: 8192,
        iterations: 3,
        parallelism: 4,
      });
      expect(mockInvoke).toHaveBeenCalledWith('derive_key', {
        password: 'pwd',
        salt: [1, 2],
        memoryKib: 8192,
        iterations: 3,
        parallelism: 4,
      });
    });
  });

  describe('OCR', () => {
    it('ocr_scan_image 传递 filePath 和可选的 language', async () => {
      const result = { text: 'hello', confidence: 0.9, boxes: [] };
      mockInvoke.mockResolvedValue(result);
      const res = (await invoke('ocr_scan_image', { filePath: '/img.png', language: 'en' })) as {
        text: string;
      };
      expect(mockInvoke).toHaveBeenCalledWith('ocr_scan_image', {
        filePath: '/img.png',
        language: 'en',
      });
      expect(res.text).toBe('hello');
    });

    it('ocr_scan_mrz 传递 filePath', async () => {
      mockInvoke.mockResolvedValue(null);
      const result = await invoke('ocr_scan_mrz', { filePath: '/mrz.png' });
      expect(result).toBeNull();
    });

    it('OCR 安装模型命令传递 tier', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('ocr_install_bundled_model', { tier: 'small' });
      expect(mockInvoke).toHaveBeenCalledWith('ocr_install_bundled_model', { tier: 'small' });
      await invoke('ocr_download_model', { tier: 'tiny', baseUrl: 'https://example.com/model' });
      expect(mockInvoke).toHaveBeenCalledWith('ocr_download_model', {
        tier: 'tiny',
        baseUrl: 'https://example.com/model',
      });
    });
  });
});
