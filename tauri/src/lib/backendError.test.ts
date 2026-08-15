import { describe, it, expect, vi, beforeEach } from 'vitest';

// i18n 实例在 lib/i18n 初始化时可能读取真实 locale 资源（较重），
// 单测只需验证 backendError.ts 的解析/翻译分支，故 mock 一个最小 t 实现。
vi.mock('@/lib/i18n', () => ({
  default: {
    // 真实调用：exists(key, { ns })，key 不带命名空间前缀
    exists: vi.fn((key: string) => {
      return key === 'sync_err_handshake_failed' || key === 'sync_err_handshake_vault_locked';
    }),
    t: vi.fn((key: string, opts?: Record<string, unknown>) => {
      // 模拟 i18next 的 {{detail}} 插值，保证断言能看到最终渲染文本。
      // 注意两种调用形态：translateSyncHandshakeDetail 用带命名空间前缀的
      // 'settings:sync_err_handshake_vault_locked'；resolveBackendErrorMessage 用
      // 'sync_err_handshake_failed' + { ns: 'settings' }。
      const normalized = key.startsWith('settings:') ? key : `settings:${key}`;
      const detail = typeof opts?.detail === 'string' ? opts.detail : '';
      if (normalized === 'settings:sync_err_handshake_vault_locked') {
        return '保险库已锁定，请先解锁保险库后再与设备同步';
      }
      if (normalized === 'settings:sync_err_handshake_failed') {
        return `与设备握手失败：${detail}`;
      }
      return key;
    }),
  },
}));

import { resolveBackendErrorMessage, translateRustError } from './backendError';

describe('translateRustError (P029-R1: password_too_short 映射)', () => {
  it('maps password-length Rust error to existing settings key (not missing common key)', () => {
    // 旧映射指向 common:password_too_short（双语 common.json 均无此键）→ 渲染裸键名；
    // 修正后指向 settings:password_too_short（settings.json 已存在）。
    expect(translateRustError('Password must be at least 8 characters')).toBe(
      'settings:password_too_short',
    );
  });

  it('unmatched errors still return null', () => {
    expect(translateRustError('Some unknown rust error')).toBeNull();
  });
});

describe('resolveBackendErrorMessage handshake detail i18n', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('translates vault is locked detail in handshake_failed', () => {
    const raw = '__SYNC_ERR__:handshake_failed:Vault is locked';
    const msg = resolveBackendErrorMessage(raw);
    // vault 锁定 detail 被翻译，且不再含英文原文
    expect(msg).not.toContain('Vault is locked');
    expect(msg).toContain('保险库已锁定');
  });

  it('translates Vault is not unlocked variant', () => {
    const raw = '__SYNC_ERR__:handshake_failed:Vault is not unlocked';
    const msg = resolveBackendErrorMessage(raw);
    expect(msg).not.toContain('not unlocked');
    expect(msg).toContain('保险库已锁定');
  });

  it('keeps unknown handshake detail verbatim', () => {
    const raw = '__SYNC_ERR__:handshake_failed:Peer fingerprint mismatch';
    const msg = resolveBackendErrorMessage(raw);
    // 未识别模式保留原文透传（既有语义）
    expect(msg).toContain('Peer fingerprint mismatch');
  });

  it('keeps connect_failed behavior unchanged', () => {
    const raw = '__SYNC_ERR__:connect_failed:Connection refused (os error 61)';
    const msg = resolveBackendErrorMessage(raw);
    expect(msg).toContain('Connection refused');
  });
});
