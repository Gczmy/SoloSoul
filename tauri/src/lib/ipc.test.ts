import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('ipc commands', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  afterEach(() => {
    vi.resetModules();
  });

  describe('Auth', () => {
    it('checkHasAccount 调用 invoke("check_has_account")', async () => {
      mockInvoke.mockResolvedValue(true);
      const { commands } = await import('./ipc');
      const result = await commands.checkHasAccount();
      expect(mockInvoke).toHaveBeenCalledWith('check_has_account');
      expect(result).toBe(true);
    });

    it('bootstrap 传递 accountName / password / locale / passwordHint', async () => {
      mockInvoke.mockResolvedValue({ id: 'acc-1', name: 'Test' });
      const { commands } = await import('./ipc');
      const result = await commands.bootstrap('Test', 'pwd123', 'zh', 'hint');
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
      const { commands } = await import('./ipc');
      await commands.bootstrap('Test', 'pwd123', 'en');
      expect(mockInvoke).toHaveBeenCalledWith('bootstrap', {
        accountName: 'Test',
        password: 'pwd123',
        locale: 'en',
        passwordHint: undefined,
      });
    });

    it('login 传递 accountId + password', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.login('acc-1', 'pwd123');
      expect(mockInvoke).toHaveBeenCalledWith('login', { accountId: 'acc-1', password: 'pwd123' });
    });

    it('logout 调用 invoke("logout")', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.logout();
      expect(mockInvoke).toHaveBeenCalledWith('logout');
    });
  });

  describe('Vault', () => {
    it('vaultUnlock 传递 accountId + password', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.vaultUnlock('acc-1', 'pwd');
      expect(mockInvoke).toHaveBeenCalledWith('unlock', { accountId: 'acc-1', password: 'pwd' });
    });

    it('vaultLock 调用 invoke("lock")', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.vaultLock();
      expect(mockInvoke).toHaveBeenCalledWith('lock');
    });

    it('vaultGetState 返回 VaultStateStr', async () => {
      mockInvoke.mockResolvedValue('unlocked');
      const { commands } = await import('./ipc');
      const result = await commands.vaultGetState();
      expect(result).toBe('unlocked');
    });

    it('vaultChangePassword 传递旧/新密码', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.vaultChangePassword('acc-1', 'old', 'new');
      expect(mockInvoke).toHaveBeenCalledWith('change_password', {
        accountId: 'acc-1',
        oldPassword: 'old',
        newPassword: 'new',
      });
    });

    it('vaultDeleteAccount 传递 accountId + password', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.vaultDeleteAccount('acc-1', 'pwd');
      expect(mockInvoke).toHaveBeenCalledWith('delete_account', {
        accountId: 'acc-1',
        password: 'pwd',
      });
    });

    it('vaultListAccounts 返回 AccountInfo[]', async () => {
      const accounts = [
        { id: '1', name: 'A' },
        { id: '2', name: 'B' },
      ];
      mockInvoke.mockResolvedValue(accounts);
      const { commands } = await import('./ipc');
      const result = await commands.vaultListAccounts();
      expect(result).toHaveLength(2);
    });
  });

  describe('Profile', () => {
    it('profileSave 传递 payload 对象', async () => {
      mockInvoke.mockResolvedValue({
        id: 'p1',
        name: 'P',
        createdAt: '',
        updatedAt: '',
        version: 1,
      });
      const { commands } = await import('./ipc');
      await commands.profileSave('acc-1', 'P', [1, 2, 3]);
      expect(mockInvoke).toHaveBeenCalledWith('profile_save', {
        payload: { accountId: 'acc-1', name: 'P', data: [1, 2, 3] },
      });
    });

    it('profileLoad 返回 Profile | null', async () => {
      mockInvoke.mockResolvedValue(null);
      const { commands } = await import('./ipc');
      const result = await commands.profileLoad('acc-1');
      expect(result).toBeNull();
    });
  });

  describe('Crypto', () => {
    it('encryptBytes / decryptBytes 传递 data', async () => {
      const { commands } = await import('./ipc');
      mockInvoke.mockResolvedValue([1, 2, 3]);
      const enc = await commands.encryptBytes([10, 20]);
      expect(mockInvoke).toHaveBeenCalledWith('encrypt_bytes', { data: [10, 20] });
      expect(enc).toEqual([1, 2, 3]);

      mockInvoke.mockResolvedValue([10, 20]);
      const dec = await commands.decryptBytes([1, 2, 3]);
      expect(dec).toEqual([10, 20]);
    });

    it('encryptWithKey / decryptWithKey 传递 key + plaintext/ciphertext', async () => {
      const { commands } = await import('./ipc');
      mockInvoke.mockResolvedValue([99]);
      await commands.encryptWithKey([1], [2]);
      expect(mockInvoke).toHaveBeenCalledWith('encrypt_with_key', { key: [1], plaintext: [2] });
      await commands.decryptWithKey([1], [99]);
      expect(mockInvoke).toHaveBeenCalledWith('decrypt_with_key', { key: [1], ciphertext: [99] });
    });

    it('deriveKey 传递所有 KDF 参数', async () => {
      const { commands } = await import('./ipc');
      mockInvoke.mockResolvedValue([1, 2, 3]);
      await commands.deriveKey('pwd', [1, 2], 8192, 3, 4);
      expect(mockInvoke).toHaveBeenCalledWith('derive_key', {
        password: 'pwd',
        salt: [1, 2],
        memoryKib: 8192,
        iterations: 3,
        parallelism: 4,
      });
    });

    it('generateSalt 传递 length', async () => {
      const { commands } = await import('./ipc');
      mockInvoke.mockResolvedValue([1, 2, 3, 4]);
      const salt = await commands.generateSalt(16);
      expect(mockInvoke).toHaveBeenCalledWith('generate_salt', { length: 16 });
      expect(salt).toHaveLength(4);
    });

    it('constantTimeCompare 返回 boolean', async () => {
      const { commands } = await import('./ipc');
      mockInvoke.mockResolvedValue(true);
      const result = await commands.constantTimeCompare([1], [1]);
      expect(result).toBe(true);
    });

    it('getVaultStats 返回统计', async () => {
      const stats = { profileCount: 5, totalSizeBytes: 1000, lastModified: '2026-01-01' };
      mockInvoke.mockResolvedValue(stats);
      const { commands } = await import('./ipc');
      const result = await commands.getVaultStats();
      expect(result).toEqual(stats);
    });
  });

  describe('File System', () => {
    it('inspectBackup 传递 backupPath', async () => {
      mockInvoke.mockResolvedValue('Backup info');
      const { commands } = await import('./ipc');
      const result = await commands.inspectBackup('/path/to/backup');
      expect(mockInvoke).toHaveBeenCalledWith('inspect_backup', { backupPath: '/path/to/backup' });
      expect(result).toBe('Backup info');
    });
  });

  describe('Discovery', () => {
    it('mdnsDiscover 传递 timeoutMs', async () => {
      mockInvoke.mockResolvedValue([]);
      const { commands } = await import('./ipc');
      const result = await commands.mdnsDiscover(5000);
      expect(mockInvoke).toHaveBeenCalledWith('mdns_discover', { timeoutMs: 5000 });
      expect(result).toEqual([]);
    });

    it('mdnsAdvertise 传递 deviceName + port', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.mdnsAdvertise('MyMac', 5432);
      expect(mockInvoke).toHaveBeenCalledWith('mdns_advertise', {
        deviceName: 'MyMac',
        port: 5432,
      });
    });
  });

  describe('Sync', () => {
    it('syncGetStatus 调用 invoke("sync_get_status")', async () => {
      const status = {
        isDiscovering: false,
        syncEnabled: true,
        localFingerprint: 'fp',
        connectedPeers: [],
      };
      mockInvoke.mockResolvedValue(status);
      const { commands } = await import('./ipc');
      const result = await commands.syncGetStatus();
      expect(result.connectedPeers).toEqual([]);
    });

    it('syncEnable 传递 enable', async () => {
      mockInvoke.mockResolvedValue(undefined);
      const { commands } = await import('./ipc');
      await commands.syncEnable(true);
      expect(mockInvoke).toHaveBeenCalledWith('sync_enable', { enable: true });
    });

    it('syncWithDevice 传递 deviceId', async () => {
      const result = {
        summary: 'ok',
        examined: 1,
        applied: 1,
        skipped: 0,
        conflicts: [],
        per_table: [],
      };
      mockInvoke.mockResolvedValue(result);
      const { commands } = await import('./ipc');
      const res = await commands.syncWithDevice('device-1');
      expect(mockInvoke).toHaveBeenCalledWith('sync_with_device', { deviceId: 'device-1' });
      expect(res.applied).toBe(1);
    });
  });

  describe('OCR', () => {
    it('ocrScanImage 传递 filePath 和可选的 language', async () => {
      const result = { text: 'hello', confidence: 0.9, boxes: [] };
      mockInvoke.mockResolvedValue(result);
      const { commands } = await import('./ipc');
      const res = await commands.ocrScanImage('/img.png', 'en');
      expect(mockInvoke).toHaveBeenCalledWith('ocr_scan_image', {
        filePath: '/img.png',
        language: 'en',
      });
      expect(res.text).toBe('hello');
    });

    it('ocrScanMrz 传递 filePath', async () => {
      mockInvoke.mockResolvedValue(null);
      const { commands } = await import('./ipc');
      const result = await commands.ocrScanMrz('/mrz.png');
      expect(result).toBeNull();
    });

    it('OCR 安装模型命令传递 tier', async () => {
      const { commands } = await import('./ipc');
      mockInvoke.mockResolvedValue(undefined);
      await commands.ocrInstallBundledModel('small');
      expect(mockInvoke).toHaveBeenCalledWith('ocr_install_bundled_model', { tier: 'small' });
      await commands.ocrDownloadModel('tiny', 'https://example.com/model');
      expect(mockInvoke).toHaveBeenCalledWith('ocr_download_model', {
        tier: 'tiny',
        baseUrl: 'https://example.com/model',
      });
    });
  });
});
