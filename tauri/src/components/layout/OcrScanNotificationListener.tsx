import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { useUiStore } from '@/stores/uiStore';
import { translateOcrError } from '@/lib/ocrScanMessages';

/** 消费 Store 已接纳的用户操作终态；MRZ 中间任务、取消请求和锁定清空都不是完成。 */
export function OcrScanNotificationListener() {
  const { t } = useTranslation('ocr');
  const showToast = useUiStore((s) => s.showToast);
  const completion = useOcrScanStore((s) => s.lastCompletion);
  const isCardOpen = useOcrScanStore((s) => s.isCardOpen);

  useEffect(() => {
    if (!completion) return;
    const accepted = useOcrScanStore.getState().claimCompletion(completion.operationId);
    if (!accepted || isCardOpen || accepted.status === 'cancelled') return;
    if (accepted.status === 'failed') {
      showToast({
        type: 'error',
        message: `${t('scan_failed')}: ${translateOcrError(accepted.error, t)}`,
        duration: 4000,
      });
    } else {
      showToast({ type: 'success', message: t('scan_complete_notification'), duration: 3000 });
    }
  }, [completion, isCardOpen, showToast, t]);

  return null;
}
