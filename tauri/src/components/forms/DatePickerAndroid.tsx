import { useRef, type RefObject } from 'react';
import { CalendarDays } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AndroidSheet } from '@/components/android/AndroidSheet';
import { Button } from '@/components/ui/Button';
import { buildValue, SEGMENTS_CONFIG, type SegmentKey, type Segments } from './DatePicker.helpers';
import choiceStyles from '@/components/android/AndroidChoice.module.css';
import styles from './DatePickerAndroid.module.css';

/** 只替换安卓展示，所有草稿传播/日期校验仍由 DatePicker 统一管理。 */
export function DatePickerAndroid({
  value,
  segments,
  includeTime,
  disabled,
  open,
  setOpen,
  segmentRefs,
  onSegmentChange,
  onCalendarChange,
  isInvalid,
  onClear,
}: {
  value?: string;
  segments: Segments;
  includeTime?: boolean;
  disabled?: boolean;
  open: boolean;
  setOpen: (open: boolean) => void;
  segmentRefs: RefObject<Partial<Record<SegmentKey, HTMLInputElement | null>>>;
  onSegmentChange: (key: SegmentKey, value: string) => void;
  onCalendarChange: (value: string) => void;
  isInvalid: (key: SegmentKey) => boolean;
  onClear: () => void;
}) {
  const { t } = useTranslation('common');
  const trigger = useRef<HTMLButtonElement>(null);
  const title = t(includeTime ? 'date_time_picker' : 'date_picker');
  const calendar = buildValue(segments, false) || '';
  const fields = (keys: SegmentKey[]) => (
    <div
      className={styles.segments}
      style={{ gridTemplateColumns: `repeat(${keys.length},minmax(0,1fr))` }}
    >
      {keys.map((key) => (
        <label key={key}>
          {t(`date_segment_${key}`)}
          <input
            ref={(node) => {
              segmentRefs.current[key] = node;
            }}
            type="text"
            inputMode="numeric"
            autoComplete="off"
            value={segments[key]}
            maxLength={SEGMENTS_CONFIG[key].max}
            placeholder={SEGMENTS_CONFIG[key].placeholder}
            aria-invalid={isInvalid(key) || undefined}
            onFocus={(e) => e.currentTarget.select()}
            onChange={(e) => onSegmentChange(key, e.target.value)}
          />
        </label>
      ))}
    </div>
  );
  return (
    <>
      <button
        ref={trigger}
        type="button"
        className={choiceStyles.trigger}
        aria-label={title}
        aria-haspopup="dialog"
        aria-expanded={open && !disabled}
        disabled={disabled}
        onClick={() => setOpen(true)}
      >
        <span>{value || title}</span>
        <CalendarDays size={20} aria-hidden="true" />
      </button>
      {open && !disabled && (
        <AndroidSheet
          title={title}
          onClose={() => setOpen(false)}
          trigger={trigger.current}
          material="solid"
          className={choiceStyles.sheet}
          zIndex={6000}
        >
          <div className={`${choiceStyles.body} ${styles.body}`} data-android-date-scroll>
            <label className={styles.calendar}>
              <CalendarDays size={22} aria-hidden="true" />
              <span>{t('choose_calendar_date')}</span>
              <input
                type="date"
                value={calendar}
                aria-label={t('choose_calendar_date')}
                onChange={(e) => onCalendarChange(e.target.value)}
              />
            </label>
            {fields(['year', 'month', 'day'])}
            {includeTime && fields(['hour', 'minute'])}
            {value && (!calendar || buildValue(segments, includeTime) === null) && (
              <p className={styles.error} role="status">
                {t('date_draft_invalid')}
              </p>
            )}
          </div>
          <footer className={choiceStyles.actions}>
            <Button type="button" variant="secondary" onClick={onClear}>
              {t('clear')}
            </Button>
            <Button type="button" onClick={() => setOpen(false)}>
              {t('close')}
            </Button>
          </footer>
        </AndroidSheet>
      )}
    </>
  );
}
