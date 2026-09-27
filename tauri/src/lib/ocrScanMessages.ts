import type { TFunction } from 'i18next';
import { OCR_MODEL_NOT_INSTALLED_PREFIX } from '@/lib/constants';
import { ocrErrorText } from '@/lib/ocrScanOperation';

export function translateOcrError(error: unknown, t: TFunction): string {
  const message = ocrErrorText(error);
  if (message.startsWith(`${OCR_MODEL_NOT_INSTALLED_PREFIX}:`)) {
    return t('ocr:scan_model_not_installed', {
      tier: message.slice(OCR_MODEL_NOT_INSTALLED_PREFIX.length + 1),
    });
  }
  const keys: Record<string, string> = {
    __OCR_QUEUE_FULL__: 'ocr:scan_queue_full',
    __OCR_DUPLICATE_TASK__: 'ocr:scan_request_invalid',
    __OCR_INVALID_TASK_ID__: 'ocr:scan_request_invalid',
  };
  return keys[message] ? t(keys[message]) : message;
}
