import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invokeCommand } from './ipcClient';
import {
  requestAndroidGlassMenu,
  isAndroidGlassMode,
  type AndroidGlassMenuPayload,
} from './androidGlass';

vi.mock('./ipcClient', () => ({ invokeCommand: vi.fn() }));
const invoke = vi.mocked(invokeCommand);
const payload: AndroidGlassMenuPayload = {
  requestId: 'request-1',
  title: 'New',
  description: '',
  closeLabel: 'Close',
  footer: '',
  labels: { object: 'Object', page: 'Page', scan: 'Scan' },
  descriptions: { object: '', page: '', scan: '' },
  dark: false,
  reduceMotion: false,
  background: '#F9F9F3',
  foreground: '#20251F',
  secondary: '#626B62',
  accent: '#405F82',
  container: '#D9E7F8',
};
describe('Android native menu ownership', () => {
  beforeEach(() => invoke.mockReset());
  it('取消后忽略迟到动作，并允许在锁定后关闭原生窗口', async () => {
    let respond!: (value: unknown) => void;
    invoke.mockImplementation((command) =>
      command === 'android_show_glass_menu'
        ? new Promise((resolve) => {
            respond = resolve;
          })
        : Promise.resolve(),
    );
    const request = requestAndroidGlassMenu(payload);
    request.cancel();
    request.cancel();
    respond({ requestId: 'request-1', action: 'object' });
    await expect(request.result).resolves.toBeNull();
    expect(invoke).toHaveBeenCalledWith(
      'android_close_glass_menu',
      { requestId: 'request-1' },
      { requireUnlocked: false },
    );
    expect(invoke.mock.calls.filter(([name]) => name === 'android_close_glass_menu')).toHaveLength(
      1,
    );
  });
  it('不执行不属于本次请求的动作', async () => {
    invoke.mockResolvedValue({ requestId: 'old-request', action: 'page' });
    await expect(requestAndroidGlassMenu(payload).result).resolves.toBeNull();
  });
  it('拒绝原生协议之外的动作', async () => {
    invoke.mockResolvedValue({ requestId: 'request-1', action: '/arbitrary-route' });
    await expect(requestAndroidGlassMenu(payload).result).rejects.toThrow(
      'Invalid Android menu action',
    );
  });
  it('保留无法提供原生材质的回退结果', async () => {
    invoke.mockResolvedValue({ requestId: 'request-1', action: 'unavailable' });
    await expect(requestAndroidGlassMenu(payload).result).resolves.toBe('unavailable');
    expect(isAndroidGlassMode('enhanced')).toBe(true);
    expect(isAndroidGlassMode('unknown')).toBe(false);
  });
});
