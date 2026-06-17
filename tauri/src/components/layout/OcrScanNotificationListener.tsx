import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { useUiStore } from '@/stores/uiStore';

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
        showToast({ type: 'error', message: `${t('ocr:scan_failed')}: ${lastScanError}`, duration: 4000 });
      } else {
        showToast({ type: 'success', message: t('ocr:scan_complete_notification'), duration: 3000 });
      }
    }

    prevScanningRef.current = nowScanning;
  }, [isScanning, isCardOpen, lastScanError, showToast, t]);

  return null;
}
