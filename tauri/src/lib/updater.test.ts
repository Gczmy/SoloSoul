import { describe, it, expect, vi, beforeEach } from 'vitest';
import { checkForUpdate, downloadAndInstallUpdate } from './updater';

const mockCheck = vi.fn();
const mockRelaunch = vi.fn();

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: () => mockCheck(),
}));

vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: () => mockRelaunch(),
}));

describe('updater', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('checkForUpdate', () => {
    it('returns available result when an update is available', async () => {
      mockCheck.mockResolvedValue({
        version: '2.0.0',
        body: 'Release notes',
        date: '2026-06-13',
      });

      const result = await checkForUpdate();

      expect(result).toEqual({
        kind: 'available',
        info: {
          version: '2.0.0',
          body: 'Release notes',
          date: '2026-06-13',
        },
      });
    });

    it('returns up-to-date result when no update is available', async () => {
      mockCheck.mockResolvedValue(null);

      const result = await checkForUpdate();

      expect(result).toEqual({ kind: 'up-to-date' });
    });

    it('returns error result and logs warning when check throws', async () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      mockCheck.mockRejectedValue(new Error('network error'));

      const result = await checkForUpdate();

      expect(result).toEqual({
        kind: 'error',
        message: 'network error',
      });
      expect(consoleSpy).toHaveBeenCalledWith('[updater] check failed:', expect.any(Error));
      consoleSpy.mockRestore();
    });
  });

  describe('downloadAndInstallUpdate', () => {
    it('downloads, installs and relaunches when update is available', async () => {
      const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
      mockCheck.mockResolvedValue({
        version: '2.0.0',
        downloadAndInstall,
      });

      await downloadAndInstallUpdate();

      expect(downloadAndInstall).toHaveBeenCalledWith(expect.any(Function));
      expect(mockRelaunch).toHaveBeenCalled();
    });

    it('throws when no update is available', async () => {
      mockCheck.mockResolvedValue(null);

      await expect(downloadAndInstallUpdate()).rejects.toThrow('No update available');
      expect(mockRelaunch).not.toHaveBeenCalled();
    });
  });
});
