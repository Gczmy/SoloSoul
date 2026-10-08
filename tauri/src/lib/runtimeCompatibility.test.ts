import { describe, expect, it, vi } from 'vitest';
import { installRuntimeCompatibility } from './runtimeCompatibility';

describe('旧 WebView 标准 API 兼容', () => {
  it('保留可用的原生实现', () => {
    const hasOwn = vi.fn();
    const randomUUID = vi.fn();
    const objectApi = { hasOwn } as unknown as typeof Object;
    const cryptoApi = { randomUUID } as unknown as Crypto;
    installRuntimeCompatibility(objectApi, cryptoApi);
    expect(objectApi.hasOwn).toBe(hasOwn);
    expect(cryptoApi.randomUUID).toBe(randomUUID);
  });

  it('正确区分继承属性、空原型、符号与被覆盖的 hasOwnProperty', () => {
    const objectApi = {} as typeof Object;
    installRuntimeCompatibility(objectApi, undefined);
    const symbol = Symbol('field');
    const value = Object.assign(Object.create({ inherited: true }), {
      own: undefined,
      hasOwnProperty: null,
      [symbol]: true,
    });
    expect(objectApi.hasOwn(value, 'inherited')).toBe(false);
    expect(objectApi.hasOwn(value, 'own')).toBe(true);
    expect(objectApi.hasOwn(value, symbol)).toBe(true);
    expect(objectApi.hasOwn(Object.create(null), 'toString')).toBe(false);
    expect(objectApi.hasOwn('abc' as unknown as object, 1)).toBe(true);
    expect(() => objectApi.hasOwn(null as unknown as object, 'field')).toThrow(TypeError);
    expect(Object.keys(objectApi)).toEqual([]);
  });

  it('使用安全随机字节并设置 UUID 的版本和变体，重复安装保持同一实现', () => {
    const random = vi.fn((bytes: Uint8Array) => {
      bytes.fill(255);
      return bytes;
    });
    const cryptoApi = { getRandomValues: random } as unknown as Crypto;
    installRuntimeCompatibility(Object, cryptoApi);
    const installed = cryptoApi.randomUUID;
    expect(cryptoApi.randomUUID()).toBe('ffffffff-ffff-4fff-bfff-ffffffffffff');
    expect(random).toHaveBeenCalledTimes(1);
    expect(random.mock.calls[0][0]).toHaveLength(16);
    installRuntimeCompatibility(Object, cryptoApi);
    expect(cryptoApi.randomUUID).toBe(installed);
  });

  it('安全随机源异常时直接失败，不退化为非安全 ID', () => {
    const cryptoApi = {
      getRandomValues: () => {
        throw new Error('random unavailable');
      },
    } as unknown as Crypto;
    installRuntimeCompatibility(Object, cryptoApi);
    expect(() => cryptoApi.randomUUID()).toThrow('random unavailable');
  });
});
