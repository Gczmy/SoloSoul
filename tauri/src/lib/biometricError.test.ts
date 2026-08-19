import { describe, it, expect, vi } from 'vitest';

// i18n 依赖只用到 t 函数：mock 最小实现。
vi.mock('@/lib/i18n', () => ({
  default: {
    t: vi.fn(() => ''),
  },
}));

import type { TFunction } from 'i18next';

import { getBiometricErrorMessage } from './biometricError';

// 模拟 react-i18next 的 t：已配置 key 返回 key 本身（可断言），
// 未配置 key 返回调用方传入的 defaultValue（getBiometricErrorMessage 传 ''），
// 无 defaultValue 时返回 key（i18next 兜底行为）。
// TFunction 带私有品牌属性，函数签名兼容需显式转型。
const KNOWN_KEYS = new Set([
  'common:password_locked',
  'settings:current_password_incorrect',
  'settings:biometric_error_invalid_password',
  'settings:biometric_error_cancelled',
  'common:error',
]);
const t: TFunction = ((key: string, opts?: Record<string, unknown>) => {
  if (KNOWN_KEYS.has(key)) return key;
  return (opts?.defaultValue as string | undefined) ?? key;
}) as unknown as TFunction;

describe('getBiometricErrorMessage (P012: 主密码阶梯锁定映射)', () => {
  it('maps the master-password lockout raw string to common:password_locked', () => {
    // 后端走 verify_password_with_lockout 时锁定错误原样上抛（'Too many failed
    // attempts; try again later'），前缀匹配应落到密码锁定文案而非通用错误。
    const msg = getBiometricErrorMessage('Too many failed attempts; try again later', t);
    expect(msg).toBe('common:password_locked');
  });

  it('maps __BIO_ERR__:invalid_password via its locale key', () => {
    // locales 中已配置 settings:biometric_error_invalid_password，按 code 查找命中。
    const msg = getBiometricErrorMessage('__BIO_ERR__:invalid_password', t);
    expect(msg).toBe('settings:biometric_error_invalid_password');
  });

  it('maps raw English "Invalid password" via fallback branch', () => {
    // 无前缀的旧版英文错误串走兜底分支。
    const msg = getBiometricErrorMessage('Invalid password', t);
    expect(msg).toBe('settings:current_password_incorrect');
  });

  it('maps __BIO_ERR__:cancelled via its locale key', () => {
    const msg = getBiometricErrorMessage('__BIO_ERR__:cancelled', t);
    expect(msg).toBe('settings:biometric_error_cancelled');
  });

  it('falls back to generic error for unknown messages', () => {
    const msg = getBiometricErrorMessage('__BIO_ERR__:save_failed', t);
    expect(msg).toBe('common:error');
  });
});
