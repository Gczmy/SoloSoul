import { describe, it, expect, vi } from 'vitest';
vi.mock('./i18n', async () => {
  const { createInstance } = await import('i18next');
  const zh = (await import('../locales/zh-CN/common.json')).default;
  const en = (await import('../locales/en-US/common.json')).default;
  const instance = createInstance();
  await instance.init({
    lng: 'zh-CN',
    fallbackLng: 'en-US',
    defaultNS: 'common',
    resources: { 'zh-CN': { common: zh }, 'en-US': { common: en } },
    interpolation: { escapeValue: false },
  });
  return { default: instance };
});
import i18n from './i18n';
import fixtures from '../../src-tauri/src/commands/object/tests/rf307-fixtures.json';
import { resolveBackendErrorMessage, translateRustError } from './backendError';
import {
  BackendCommandError,
  readBackendError,
  normalizeObjectError,
  backendErrorLogDetails,
} from './backendErrorWire';
const SECRET = 'RF307-SYNTHETIC-SECRET-FIELD-key-path';

describe('RF307 wire and display separation', () => {
  it('actual Host serde fixtures localize in both languages, including the existing toast key entry', async () => {
    for (const language of ['zh-CN', 'en-US']) {
      await i18n.changeLanguage(language);
      for (const fixture of Object.values(fixtures)) {
        const decoded = readBackendError(fixture);
        expect(decoded).toEqual(fixture);
        const error = new BackendCommandError(decoded!);
        expect(error.message).toBe(fixture.code);
        const translated = resolveBackendErrorMessage(error);
        expect(translated).not.toContain(fixture.code);
        expect(translated).not.toContain('common:');
        expect(translated).not.toContain(SECRET);
        expect(i18n.t(translateRustError(error.message)!)).toBe(translated);
        expect(Object.hasOwn(error, 'cause')).toBe(false);
      }
    }
    await i18n.changeLanguage('zh-CN');
  });
  it('changing language after failure translates again without rewriting stored machine details', async () => {
    const error = new BackendCommandError(normalizeObjectError(fixtures.rollbackMismatch));
    await i18n.changeLanguage('zh-CN');
    const zh = resolveBackendErrorMessage(error);
    await i18n.changeLanguage('en-US');
    const en = resolveBackendErrorMessage(error);
    expect(zh).not.toBe(en);
    expect(error.message).toBe('SNAPSHOT_OWNERSHIP_MISMATCH');
    expect(error.backend).toEqual(fixtures.rollbackMismatch);
    await i18n.changeLanguage('zh-CN');
  });
  it('whitelists details, strips cause/field/path and rejects inherited or malformed contract fields', () => {
    const polluted = {
      ...fixtures.longName,
      cause: SECRET,
      message: SECRET,
      safeDetails: { ...fixtures.longName.safeDetails, path: SECRET, field: SECRET },
    };
    expect(readBackendError(polluted)).toEqual(fixtures.longName);
    expect(JSON.stringify(backendErrorLogDetails(polluted))).not.toContain(SECRET);
    for (const bad of [
      { ...fixtures.longName, code: SECRET },
      { ...fixtures.longName, retryable: 'true' },
      { ...fixtures.longName, safeDetails: { stage: SECRET } },
      { ...fixtures.longName, safeDetails: SECRET },
      Object.create(fixtures.longName),
      { code: 'OBJECT_NOT_FOUND', retryable: false },
    ]) {
      expect(readBackendError(bad)).toBeNull();
      expect(normalizeObjectError(bad)).toEqual({
        code: 'INTERNAL_ERROR',
        safeDetails: null,
        retryable: false,
      });
    }
    expect(
      readBackendError({ ...fixtures.longName, safeDetails: { stage: 'validate', limit: 123456 } })
        ?.safeDetails,
    ).toEqual({ stage: 'validate' });
  });
  it('unknown structured codes and unknown object failures use safe fallback; other legacy domains remain readable', () => {
    expect(
      resolveBackendErrorMessage({ code: SECRET, safeDetails: { path: SECRET }, retryable: true }),
    ).toBe(i18n.t('common:backend_operation_failed'));
    expect(
      resolveBackendErrorMessage(new BackendCommandError(normalizeObjectError(new Error(SECRET)))),
    ).toBe(i18n.t('common:backend_operation_failed'));
    expect(resolveBackendErrorMessage('Invalid password')).toBe(i18n.t('common:invalid_password'));
    expect(backendErrorLogDetails(new Error(SECRET))).toEqual({ code: 'LEGACY_ERROR' });
  });
  it.each([
    ['对象名称不能为空', 'OBJECT_NAME_REQUIRED'],
    ['对象名称不能超过 200 字符', 'OBJECT_NAME_TOO_LONG'],
    ['对象属性载荷过大（超过 10 MiB）', 'OBJECT_PAYLOAD_TOO_LARGE'],
    ['Object not found', 'OBJECT_NOT_FOUND'],
    ['Object has no associated template', 'OBJECT_TEMPLATE_MISSING'],
    ['Template not found', 'OBJECT_TEMPLATE_NOT_FOUND'],
    ['Snapshot not found', 'SNAPSHOT_NOT_FOUND'],
    ['Snapshot does not belong to object', 'SNAPSHOT_OWNERSHIP_MISMATCH'],
    [`Object with ID '${SECRET}' already exists`, 'OBJECT_ID_EXISTS'],
    [`字段 '${SECRET}' 是动态字段组，其值必须是数组`, 'OBJECT_VALIDATION_FAILED'],
    ['Request belongs to an expired session', 'SESSION_EXPIRED'],
  ])('adapts legacy object error %s to %s without dynamic details', (raw, code) => {
    const decoded = normalizeObjectError(new Error(raw));
    expect(decoded.code).toBe(code);
    const error = new BackendCommandError(decoded);
    expect(JSON.stringify(error)).not.toContain(SECRET);
    expect(resolveBackendErrorMessage(error)).not.toContain(SECRET);
  });
});
