import { useTranslation } from 'react-i18next';
import { useStore } from 'zustand';
import { Button } from '@/components/ui/Button';
import { useOcrScanCapability, ocrCapabilityMessageKey } from '@/lib/ocrCapabilities';
import { getPlatformCapabilities, platformCapabilityStore } from '@/lib/platformCapabilities';

/** 不支持的实现仅说明原因；暂不可用的桥接允许重新探测，恢复后所有入口同步更新。 */
export function OcrCapabilityNotice({ messageKey }: { messageKey?: string }) {
  const { t } = useTranslation(['ocr', 'common']);
  const capability = useOcrScanCapability();
  const reading = useStore(platformCapabilityStore, (state) => state.pending !== null);
  if (capability.status === 'supported') return null;
  return (
    <div>
      <p role="status">{t(messageKey ?? ocrCapabilityMessageKey(capability))}</p>
      {capability.status === 'unavailable' && (
        <Button
          variant="secondary"
          size="sm"
          loading={reading}
          onClick={() => void getPlatformCapabilities(true)}
        >
          {t('common:retry')}
        </Button>
      )}
    </div>
  );
}
