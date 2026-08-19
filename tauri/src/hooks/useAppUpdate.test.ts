import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useAppUpdate } from './useAppUpdate';
import { useAuthStore } from '@/stores/authStore';

vi.mock('@/lib/platform', () => ({
  isMobilePlatformSync: () => true,
}));

const mockAndroidCheckForUpdate = vi.fn();
const mockCheckForUpdate = vi.fn();
vi.mock('@/lib/updater', () => ({
  androidCheckForUpdate: () => mockAndroidCheckForUpdate(),
  checkForUpdate: () => mockCheckForUpdate(),
  ensureApkDownloaded: vi.fn(),
  androidInstallApk: vi.fn(),
}));

describe('useAppUpdate（Android 横幅：P027 豁免 + 解锁后重查）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // 默认锁定
    useAuthStore.setState({ isAuthenticated: false });
    mockAndroidCheckForUpdate.mockResolvedValue({ kind: 'up-to-date' });
  });

  it('挂载时（未解锁）即发起 Android 更新检查——不被 P027 守卫拦截', () => {
    renderHook(() => useAppUpdate());
    expect(mockAndroidCheckForUpdate).toHaveBeenCalledTimes(1);
  });

  it('解锁完成后重查一次（挂载期失败/被拦截的兜底），横幅在解锁后必然出现', async () => {
    mockAndroidCheckForUpdate.mockResolvedValue({
      kind: 'available',
      info: {
        latestVersion: '2.11.1',
        currentVersion: '2.11.0',
        downloadUrl: 'https://example.com/app.apk',
        checksum: '',
        checksumWarning: null,
        mandatory: false,
        releaseNotes: 'notes',
        publishedAt: null,
        apkSize: 100,
      },
    });
    const { result } = renderHook(() => useAppUpdate());
    expect(mockAndroidCheckForUpdate).toHaveBeenCalledTimes(1); // 挂载（锁定）检查

    useAuthStore.setState({ isAuthenticated: true });
    await waitFor(() => {
      expect(mockAndroidCheckForUpdate).toHaveBeenCalledTimes(2); // 解锁后重查
    });
    expect(result.current.updateState.kind).toBe('available');
  });
});
