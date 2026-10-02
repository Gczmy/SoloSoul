import { capabilityFixture } from './__fixtures__/platformCapabilityFixture';
import { describe, expect, it, vi } from 'vitest';
import { createPlatformCapabilityStore, readPlatformCapabilities } from './platformCapabilities';
import type { PlatformCapabilities, PlatformCapability } from './generated/ipcContracts';

vi.mock('./typedIpc', () => ({ invokeTypedCommand: vi.fn() }));
const supported: PlatformCapability = {
  status: 'supported',
  reason: null,
  implementation: 'test_bridge',
};
const desktop: PlatformCapabilities = {
  os: 'macos',
  updateMethod: 'tauri',
  update: supported,
  ocr: supported,
  nativeMaterial: supported,
  biometric: supported,
  fileOpen: supported,
};

function assertDisabled(c: PlatformCapabilities) {
  expect(c.updateMethod).toBe('none');
  for (const field of [c.update, c.ocr, c.nativeMaterial, c.biometric, c.fileOpen]) {
    expect(field.status).not.toBe('supported');
    expect(field.reason).toBeTruthy();
  }
}

describe('RF-205 能力读取', () => {
  it('启动未知时全部禁用；并发读取只创建一个 IPC 请求，成功快照跨页复用', async () => {
    const request = vi.fn().mockResolvedValue(desktop);
    const store = createPlatformCapabilityStore(request);
    assertDisabled(store.getState().capabilities);
    const first = store.getState().load();
    expect(store.getState().load()).toBe(first);
    expect(await first).toEqual(desktop);
    expect(await store.getState().load()).toEqual(desktop);
    expect(request).toHaveBeenCalledOnce();
    expect(store.getState().pending).toBeNull();
  });

  it('缺命令/桥接故障不会变成支持，也不会永久缓存失败', async () => {
    const request = vi
      .fn()
      .mockRejectedValueOnce(new Error('missing command'))
      .mockResolvedValueOnce(desktop);
    const store = createPlatformCapabilityStore(request);
    assertDisabled(await store.getState().load());
    expect(store.getState().capabilities.update.reason).toBe('capabilities_read_failed');
    expect(store.getState().loaded).toBe(false);
    expect(await store.getState().load()).toEqual(desktop);
    expect(request).toHaveBeenCalledTimes(2);
  });

  it('显式刷新可使原本可用的桥接变成不可用，页面读取最新快照', async () => {
    const unavailable = {
      status: 'unavailable',
      reason: 'ocr_bridge_unavailable',
      implementation: 'desktop_ocr',
    };
    const request = vi
      .fn()
      .mockResolvedValueOnce(desktop)
      .mockResolvedValueOnce({ ...desktop, ocr: unavailable });
    const store = createPlatformCapabilityStore(request);
    await store.getState().load();
    await store.getState().load(true);
    expect(store.getState().capabilities.ocr).toEqual(unavailable);
    expect(request).toHaveBeenCalledTimes(2);
  });

  it('未知平台、缺原因、缺桥接、畸形状态和更新方式不一致均安全禁用', () => {
    for (const wire of [
      null,
      {},
      { ...desktop, os: 'future-os' },
      { ...desktop, updateMethod: 'none' },
      { ...desktop, ocr: { status: 'unsupported', reason: null, implementation: null } },
      { ...desktop, ocr: { status: 'supported', reason: null, implementation: null } },
      { ...desktop, ocr: { ...supported, status: 'maybe' } },
    ])
      assertDisabled(readPlatformCapabilities(wire));
  });
});

describe('RF205 fixed platform contracts', () => {
  it.each(['macos', 'windows', 'linux', 'android', 'ios'])(
    '%s 固定 fixture 可完整读取且不重建能力',
    (os) => {
      const fixture = capabilityFixture(os);
      expect(readPlatformCapabilities(fixture)).toEqual(fixture);
    },
  );
  it('Android 桥接暂不可用时保留 APK 方式，能力状态仍不支持操作', () => {
    const fixture = capabilityFixture('android');
    fixture.update = {
      status: 'unavailable',
      reason: 'apk_bridge_unavailable',
      implementation: 'android_apk',
    };
    expect(readPlatformCapabilities(fixture)).toEqual(fixture);
  });
});
