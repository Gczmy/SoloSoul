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
    // P011: 遗留 unlock 命令已从 handler 面删除（生产前端零调用，改用 unlock_with_password），
    // 对应 mock 测试一并移除。

    it('lock 调用 invoke("lock")', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await invoke('lock');
      expect(mockInvoke).toHaveBeenCalledWith('lock');
    });

    // P002: get_state / delete_account 命令已从 handler 面删除（生产前端零调用），
    // 对应 mock 测试一并移除。

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

  // N-9: Crypto 块已整体移除——encrypt_bytes / decrypt_bytes / derive_key
  // 命令在 P132/P205 中已从后端删除（crypto.rs 整文件删除），这些 mock 测试
  // 针对的已是不存在的命令面。

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
