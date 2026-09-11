import { afterEach, expect, it, vi } from 'vitest';
import { withTimeout } from './withTimeout';

afterEach(() => vi.useRealTimers());

it('完成读取后清理超时计时器', async () => {
  vi.useFakeTimers();
  await expect(withTimeout(Promise.resolve('cached'), 600)).resolves.toBe('cached');
  expect(vi.getTimerCount()).toBe(0);
});

it('超时结束等待，迟到结果不会恢复已经失败的读取', async () => {
  vi.useFakeTimers();
  let finish!: (value: string) => void;
  const operation = new Promise<string>((resolve) => {
    finish = resolve;
  });
  const result = withTimeout(operation, 600);
  const rejection = expect(result).rejects.toThrow('timed out');
  await vi.advanceTimersByTimeAsync(600);
  await rejection;
  finish('late');
  await expect(result).rejects.toThrow('timed out');
});

it('原始错误立即传递，并清理计时器', async () => {
  vi.useFakeTimers();
  await expect(withTimeout(Promise.reject(new Error('offline')), 600)).rejects.toThrow('offline');
  expect(vi.getTimerCount()).toBe(0);
});
