import { afterEach, beforeEach, describe, it, expect, vi } from 'vitest';
const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => native.invoke(...args) }));
import { invokeTypedCommand } from './typedIpc';
import { resolveBackendErrorMessage, resolvePluginErrorMessage } from './backendError';
import {
  readBackendError,
  normalizePluginError,
  normalizePluginCommandError,
  readPluginEventError,
  isPluginCancellation,
  makeBackendError,
  backendErrorLogDetails,
} from './backendErrorWire';
import i18n from './i18n';
import { useAuthStore } from '@/stores/authStore';
import fixture from '../../src-tauri/src/commands/plugin/contracts/rf320-fixtures.json';
const PRIVATE = 'RF320_PRIVATE_FIELD_KEY_PATH';
beforeEach(() => {
  native.invoke.mockReset();
  useAuthStore.getState().completeUnlock({ id: 'synthetic', name: 'Synthetic' });
});
afterEach(async () => {
  await i18n.changeLanguage('en-US');
});
it('RF320 actual plugin native rejection keeps private cause out of UI', async () => {
  native.invoke.mockRejectedValueOnce(new Error(PRIVATE));
  const error = await invokeTypedCommand('plugin_list_installed').catch((e: unknown) => e);
  expect(resolveBackendErrorMessage(error)).not.toContain('RF320_PRIVATE');
  expect(readBackendError(error)?.code).toBe('PLUGIN_READ_FAILED');
});
describe('RF320 shared Rust serialization fixture', () => {
  it.each(Object.entries(fixture))(
    '%s survives native rejection without arbitrary metadata',
    async (_name, wire) => {
      native.invoke.mockRejectedValueOnce({
        ...wire,
        message: PRIVATE,
        cause: PRIVATE,
        safeDetails: { ...wire.safeDetails, fieldId: PRIVATE, key: PRIVATE },
      });
      const error = await invokeTypedCommand('plugin_list_installed').catch((e: unknown) => e);
      expect(readBackendError(error)).toEqual(wire);
      expect(JSON.stringify(error)).not.toContain(PRIVATE);
      expect(String(error)).not.toContain(PRIVATE);
      expect(JSON.stringify(backendErrorLogDetails(error))).not.toContain(PRIVATE);
    },
  );
});
const codes = [
  'PLUGIN_CHECKSUM_MISMATCH',
  'PLUGIN_CONSENT_DENIED',
  'PLUGIN_EXECUTION_FAILED',
  'PLUGIN_INVALID_ARGUMENT',
  'PLUGIN_INVALID_FIELD',
  'PLUGIN_MANIFEST_INVALID',
  'PLUGIN_NETWORK_FAILED',
  'PLUGIN_NOT_FOUND',
  'PLUGIN_RATE_LIMITED',
  'PLUGIN_REGISTRY_FAILED',
  'PLUGIN_SESSION_EXPIRED',
  'PLUGIN_STORE_FAILED',
  'PLUGIN_TASK_UNCONFIRMED',
  'PLUGIN_VERSION_INCOMPATIBLE',
  'PLUGIN_WASM_TOO_LARGE',
  'PLUGIN_INSTALL_CANCELLED',
  'PLUGIN_INSTALL_ALREADY_STARTED',
  'PLUGIN_INVALID_OPERATION',
  'PLUGIN_OUTPUT_INVALID',
  'PLUGIN_OUTPUT_DENIED',
  'PLUGIN_OUTPUT_READ_FAILED',
  'PLUGIN_OUTPUT_WRITE_FAILED',
  'PLUGIN_OUTPUT_OPEN_FAILED',
  'PLUGIN_UNSUPPORTED',
  'PLUGIN_READ_FAILED',
  'PLUGIN_INSTALL_FAILED',
] as const;
it.each(codes)('RF320 %s has real Chinese and English translations', async (code) => {
  for (const language of ['zh-CN', 'en-US']) {
    await i18n.changeLanguage(language);
    const message = resolvePluginErrorMessage(code);
    expect(message).not.toBe(code);
    expect(message).not.toContain('backend_plugin_');
    expect(message).not.toContain('common:');
    expect(message).toBe(i18n.t('common:backend_' + code.toLowerCase()));
  }
  expect(normalizePluginError(code)).toEqual(makeBackendError(code));
});
it('RF320 cancellation requires the actual token or AbortError, never a cause substring', () => {
  expect(isPluginCancellation(new DOMException('cancelled', 'AbortError'))).toBe(true);
  expect(isPluginCancellation('PLUGIN_INSTALL_CANCELLED')).toBe(true);
  expect(isPluginCancellation(makeBackendError('PLUGIN_INSTALL_CANCELLED'))).toBe(true);
  for (const raw of [
    PRIVATE + ' PLUGIN_INSTALL_CANCELLED',
    'PLUGIN_INSTALL_CANCELLED ' + PRIVATE,
    new Error(PRIVATE + ' AbortError'),
    { message: 'PLUGIN_INSTALL_CANCELLED' },
  ])
    expect(isPluginCancellation(raw)).toBe(false);
});
it.each([
  ['plugin_run', 'PLUGIN_EXECUTION_FAILED'],
  ['plugin_install', 'PLUGIN_INSTALL_FAILED'],
  ['plugin_update', 'PLUGIN_INSTALL_FAILED'],
  ['plugin_uninstall', 'PLUGIN_STORE_FAILED'],
  ['plugin_update_registry', 'PLUGIN_REGISTRY_FAILED'],
  ['plugin_list_attachments', 'PLUGIN_READ_FAILED'],
  ['plugin_open_output_file', 'PLUGIN_OUTPUT_OPEN_FAILED'],
  ['plugin_copy_output_file', 'PLUGIN_OUTPUT_WRITE_FAILED'],
  ['plugin_consent_response', 'PLUGIN_INVALID_ARGUMENT'],
] as const)(
  'RF320 unknown old %s error is classified without guessing its private text',
  (command, code) => {
    const error = normalizePluginCommandError(
      command,
      new Error(PRIVATE + ' PLUGIN_INSTALL_CANCELLED'),
    );
    expect(error.code).toBe(code);
    expect(JSON.stringify(error)).not.toContain(PRIVATE);
  },
);
it.each([
  ['用户拒绝授权', 'PLUGIN_CONSENT_DENIED'],
  ['插件执行失败: 插件会话已失效：Vault 已锁定或授权已过期', 'PLUGIN_SESSION_EXPIRED'],
  ['非法字段: ' + PRIVATE, 'PLUGIN_INVALID_FIELD'],
  ['插件未找到: ' + PRIVATE, 'PLUGIN_NOT_FOUND'],
  ['插件存储错误: ' + PRIVATE, 'PLUGIN_STORE_FAILED'],
  ['网络错误: ' + PRIVATE, 'PLUGIN_NETWORK_FAILED'],
] as const)('RF320 fixed old variant prefix %s maps centrally', (raw, code) => {
  expect(normalizePluginError(raw).code).toBe(code);
  expect(resolvePluginErrorMessage(raw)).not.toContain(PRIVATE);
});
it('RF320 channel errors admit only the fixed class, dropping cause and unknown codes', () => {
  for (const raw of [
    PRIVATE,
    JSON.stringify({ code: PRIVATE, message: PRIVATE, field: PRIVATE }),
    JSON.stringify({ code: 'PLUGIN_EXECUTION_FAILED', message: PRIVATE }),
  ]) {
    expect(readPluginEventError(raw).code).toBe('PLUGIN_EXECUTION_FAILED');
    expect(JSON.stringify(readPluginEventError(raw))).not.toContain(PRIVATE);
  }
  expect(readPluginEventError(JSON.stringify({ message: '用户拒绝授权' })).code).toBe(
    'PLUGIN_CONSENT_DENIED',
  );
});
