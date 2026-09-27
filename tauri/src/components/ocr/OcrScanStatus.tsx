import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import type { OcrJobState } from '@/lib/ocrScanOperation';

export function OcrScanStatus({
  state,
  onCancel,
}: {
  state: OcrJobState | null | undefined;
  onCancel?: () => void;
}) {
  const { t } = useTranslation('ocr');
  const labels = {
    queued: 'scan_queued',
    running: 'scanning',
    cancelRequested: 'scan_cancelling',
    cancelled: 'scan_cancelled',
  } as const;
  if (!state || !(state in labels)) return null;
  const key = labels[state as keyof typeof labels];
  return (
    <div
      style={{
        display: 'flex',
        gap: 12,
        alignItems: 'center',
        justifyContent: 'center',
        padding: 12,
      }}
    >
      <span role="status">{t(key)}</span>
      {state !== 'cancelled' && onCancel && (
        <Button
          variant="secondary"
          size="sm"
          disabled={state === 'cancelRequested'}
          onClick={onCancel}
        >
          {t('scan_cancel')}
        </Button>
      )}
    </div>
  );
}
