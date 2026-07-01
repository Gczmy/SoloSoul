import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { useUiStore } from '@/stores/uiStore';
import { OCR_MODEL_NOT_INSTALLED_PREFIX } from '@/lib/constants';

/**
 * OcrScanNotificationListener — watches for background scan completion
 * and shows a toast notification when the popover card is closed.
 */
export function OcrScanNotificationListener() {
  const { t } = useTranslation('ocr');
  const showToast = useUiStore((s) => s.showToast);
  const isScanning = useOcrScanStore((s) => s.isScanning);
  const isCardOpen = useOcrScanStore((s) => s.isCardOpen);
  const lastScanError = useOcrScanStore((s) => s.lastScanError);

  const prevScanningRef = useRef(false);

  useEffect(() => {
    const wasScanning = prevScanningRef.current;
    const nowScanning = isScanning;

    if (wasScanning && !nowScanning && !isCardOpen) {
      // Scan just finished while card was closed
      if (lastScanError) {
        let message = lastScanError;
        if (lastScanError.startsWith(`${OCR_MODEL_NOT_INSTALLED_PREFIX}:`)) {
          const tier = lastScanError.slice(OCR_MODEL_NOT_INSTALLED_PREFIX.length + 1);
          message = t('ocr:scan_model_not_installed', { tier });
        }
        showToast({
          type: 'error',
          message: `${t('ocr:scan_failed')}: ${message}`,
          duration: 4000,
        });
      } else {
        showToast({
          type: 'success',
          message: t('ocr:scan_complete_notification'),
          duration: 3000,
        });
      }
    }

    prevScanningRef.current = nowScanning;
  }, [isScanning, isCardOpen, lastScanError, showToast, t]);

  return null;
}
