import { describe, it, expect, beforeEach } from 'vitest';
import {
  LOGIN_METHOD_CACHE_KEY,
  readCachedLoginMethod,
  writeCachedLoginMethod,
  clearCachedLoginMethod,
} from './loginMethodCache';

describe('loginMethodCache（方案 A：登录方式 localStorage 持久化）', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('写入后同账户可读回', () => {
    writeCachedLoginMethod('acc-1', 'touchId');
    expect(readCachedLoginMethod('acc-1')).toBe('touchId');
  });

  it('按账户隔离——其他账户读不到', () => {
    writeCachedLoginMethod('acc-1', 'touchId');
    expect(readCachedLoginMethod('acc-2')).toBeNull();
  });

  it('未写入 / 空账户返回 null', () => {
    expect(readCachedLoginMethod('acc-1')).toBeNull();
    expect(readCachedLoginMethod('')).toBeNull();
  });

  it('accountId 或 method 为空时不写入（不清除旧值）', () => {
    writeCachedLoginMethod('acc-1', 'pin');
    writeCachedLoginMethod('', 'touchId');
    writeCachedLoginMethod('acc-1', null);
    expect(readCachedLoginMethod('acc-1')).toBe('pin');
  });

  it('损坏的 JSON 返回 null（不抛错）', () => {
    localStorage.setItem(LOGIN_METHOD_CACHE_KEY, '{broken json');
    expect(readCachedLoginMethod('acc-1')).toBeNull();
  });

  it('非法 method 值返回 null（数据完整性防御）', () => {
    localStorage.setItem(
      LOGIN_METHOD_CACHE_KEY,
      JSON.stringify({ accountId: 'acc-1', method: 'voicePrint' }),
    );
    expect(readCachedLoginMethod('acc-1')).toBeNull();
  });

  it('非对象结构返回 null', () => {
    localStorage.setItem(LOGIN_METHOD_CACHE_KEY, JSON.stringify('touchId'));
    expect(readCachedLoginMethod('acc-1')).toBeNull();
  });

  it('clearCachedLoginMethod 仅清除匹配的登录方式', () => {
    writeCachedLoginMethod('acc-1', 'pin');
    clearCachedLoginMethod('acc-1', 'pin');
    expect(readCachedLoginMethod('acc-1')).toBeNull();
  });

  it('clearCachedLoginMethod 方式不匹配时保留缓存', () => {
    writeCachedLoginMethod('acc-1', 'touchId');
    clearCachedLoginMethod('acc-1', 'pin');
    expect(readCachedLoginMethod('acc-1')).toBe('touchId');
  });

  it('clearCachedLoginMethod 空账户不操作', () => {
    writeCachedLoginMethod('acc-1', 'pin');
    clearCachedLoginMethod('', 'pin');
    expect(readCachedLoginMethod('acc-1')).toBe('pin');
  });
});
