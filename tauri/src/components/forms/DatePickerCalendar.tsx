/**
 * P008：DatePicker 日历弹层（自 DatePicker.tsx 拆出）。
 * 月份导航 / 年月下拉 / 星期表头 / 日期网格 / 时间微调输入；
 * 状态（viewDate/selectedDate）由父组件持有，本组件纯展示 + 回调上报。
 */
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  format,
  startOfMonth,
  endOfMonth,
  eachDayOfInterval,
  isSameMonth,
  isSameDay,
  startOfWeek,
  endOfWeek,
  getYear,
  getMonth,
  setHours,
  setMinutes,
  setMonth,
} from 'date-fns';
import type { Locale } from 'date-fns';
import { DropdownSelect } from '@/components/ui/DropdownSelect';
import styles from './DatePicker.module.css';

interface DatePickerCalendarProps {
  viewDate: Date;
  selectedDate?: Date;
  includeTime?: boolean;
  locale: Locale;
  onSelect: (day: Date) => void;
  onApplyValue: (value: string) => void;
  onPrevMonth: () => void;
  onNextMonth: () => void;
  onYearChange: (year: number) => void;
  onMonthChange: (month: number) => void;
}

export function DatePickerCalendar({
  viewDate,
  selectedDate,
  includeTime,
  locale,
  onSelect,
  onApplyValue,
  onPrevMonth,
  onNextMonth,
  onYearChange,
  onMonthChange,
}: DatePickerCalendarProps) {
  const { t } = useTranslation(['common']);

  const calendarDays = useMemo(() => {
    const start = startOfWeek(startOfMonth(viewDate), { weekStartsOn: 0 });
    const end = endOfWeek(endOfMonth(viewDate), { weekStartsOn: 0 });
    return eachDayOfInterval({ start, end });
  }, [viewDate]);

  const currentYear = getYear(viewDate);
  const yearOptions = useMemo(() => {
    const years: number[] = [];
    // Reasonable range for native select: covers 1900s to future
    for (let y = currentYear - 80; y <= currentYear + 10; y++) {
      years.push(y);
    }
    return years;
  }, [currentYear]);

  const today = new Date();
  const weekdayLabels = useMemo(() => {
    const base = startOfWeek(new Date(), { weekStartsOn: 0 });
    return Array.from({ length: 7 }, (_, i) => {
      const d = new Date(base);
      d.setDate(base.getDate() + i);
      return format(d, 'EEE', { locale });
    });
  }, [locale]);

  return (
    <div className={styles.popover} role="dialog" aria-label="日期选择">
      <div className={styles.header}>
        <button
          type="button"
          className={styles.navButton}
          onClick={onPrevMonth}
          aria-label="上个月"
        >
          ‹
        </button>{' '}
        <div className={styles.selects}>
          <DropdownSelect
            value={getYear(viewDate)}
            onChange={(v) => onYearChange(Number(v))}
            options={yearOptions.map((y) => ({ value: y, label: String(y) }))}
            triggerLabel={String(getYear(viewDate))}
            ariaLabel="选择年份"
          />
          <DropdownSelect
            value={getMonth(viewDate)}
            onChange={(v) => onMonthChange(Number(v))}
            options={Array.from({ length: 12 }, (_, i) => ({
              value: i,
              label: format(setMonth(new Date(2020, 0, 1), i), 'MMM', { locale }),
            }))}
            triggerLabel={format(viewDate, 'MMM', { locale })}
            ariaLabel="选择月份"
          />
        </div>
        <button
          type="button"
          className={styles.navButton}
          onClick={onNextMonth}
          aria-label="下个月"
        >
          ›
        </button>
      </div>

      <div className={styles.weekdays}>
        {weekdayLabels.map((label) => (
          <div key={label} className={styles.weekday}>
            {label}
          </div>
        ))}
      </div>

      <div className={styles.days} role="grid">
        {calendarDays.map((day) => {
          const inMonth = isSameMonth(day, viewDate);
          const selected = !!selectedDate && isSameDay(day, selectedDate);
          const isToday = isSameDay(day, today);
          return (
            <button
              key={day.toISOString()}
              type="button"
              role="gridcell"
              aria-label={format(day, 'yyyy-MM-dd')}
              aria-selected={selected}
              className={[
                styles.day,
                !inMonth && styles.otherMonth,
                selected && styles.selectedDay,
                isToday && styles.today,
              ]
                .filter(Boolean)
                .join(' ')}
              onClick={() => onSelect(day)}
            >
              {format(day, 'd')}
            </button>
          );
        })}
      </div>

      {includeTime && selectedDate && (
        <div className={styles.timeRow}>
          <input
            type="number"
            className={styles.timeInput}
            min={0}
            max={23}
            value={String(selectedDate.getHours()).padStart(2, '0')}
            onChange={(e) => {
              const h = Math.min(23, Math.max(0, Number(e.target.value) || 0));
              const next = setHours(selectedDate, h);
              onApplyValue(format(next, "yyyy-MM-dd'T'HH:mm"));
            }}
            aria-label={t('common:hour', { defaultValue: '小时' })}
          />
          <span className={styles.timeSeparator}>:</span>
          <input
            type="number"
            className={styles.timeInput}
            min={0}
            max={59}
            value={String(selectedDate.getMinutes()).padStart(2, '0')}
            onChange={(e) => {
              const m = Math.min(59, Math.max(0, Number(e.target.value) || 0));
              const next = setMinutes(selectedDate, m);
              onApplyValue(format(next, "yyyy-MM-dd'T'HH:mm"));
            }}
            aria-label={t('common:minute', { defaultValue: '分钟' })}
          />
        </div>
      )}
    </div>
  );
}
