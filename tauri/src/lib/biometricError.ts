import type { TFunction } from 'i18next';
import { tryParsePrefixedError } from './utils';

const PREFIX = '__BIO_ERR__:';

/**
 * 把生物识别相关的后端错误转换为前端可读的本地化消息。
 *
 * 后端使用 `__BIO_ERR__:<code>` 格式返回错误代码，前端根据 code 查找
 * `settings:biometric_error_<code>` 对应的文案；若无法识别则返回通用错误。
 */
export function getBiometricErrorMessage(error: unknown, t: TFunction): string {
  const msg = String(error ?? '');
  const code = tryParsePrefixedError(msg, PREFIX);
  if (code !== null) {
    const localized = t(`settings:biometric_error_${code}`, {
      defaultValue: '',
    });
    if (localized) return localized;
  }

  // 兜底：兼容旧版直接返回的英文错误字符串
  const lower = msg.toLowerCase();
  if (lower.includes('invalid password')) {
    return t('settings:current_password_incorrect');
  }
  if (lower.includes('cancel') || lower.includes('interrupted')) {
    return t('settings:biometric_error_cancelled', { defaultValue: 'Cancelled' });
  }

  return t('common:error');
}
