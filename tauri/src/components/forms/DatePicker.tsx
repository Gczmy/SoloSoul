import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  format,
  parseISO,
  startOfMonth,
  endOfMonth,
  eachDayOfInterval,
  isSameMonth,
  isSameDay,
  startOfWeek,
  endOfWeek,
  isValid,
  getYear,
  getMonth,
  setYear,
  setMonth,
  setHours,
  setMinutes,
} from 'date-fns';
import { zhCN, enUS } from 'date-fns/locale';
import { Calendar, X } from 'lucide-react';
import styles from './DatePicker.module.css';

interface DatePickerProps {
  value?: string;
  onChange: (value: string | undefined) => void;
  includeTime?: boolean;
  disabled?: boolean;
  placeholder?: string;
}

function parseValue(value?: string): Date | undefined {
  if (!value) return undefined;
  const d = parseISO(value);
  return isValid(d) ? d : undefined;
}

export function DatePicker({
  value,
  onChange,
  includeTime,
  disabled,
  placeholder,
}: DatePickerProps) {
  const { i18n } = useTranslation();
  const locale = useMemo(() => (i18n.language.startsWith('zh') ? zhCN : enUS), [i18n.language]);

  const [open, setOpen] = useState(false);
  const [viewDate, setViewDate] = useState<Date>(() => parseValue(value) || new Date());
  const [selectedDate, setSelectedDate] = useState<Date | undefined>(() => parseValue(value));
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const d = parseValue(value);
    setSelectedDate(d);
    if (d) setViewDate(d);
  }, [value]);

  const handleSelect = useCallback(
    (day: Date) => {
      let next = day;
      if (selectedDate && includeTime) {
        next = setHours(setMinutes(day, selectedDate.getMinutes()), selectedDate.getHours());
      }
      setSelectedDate(next);
      onChange(format(next, includeTime ? "yyyy-MM-dd'T'HH:mm" : 'yyyy-MM-dd'));
      if (!includeTime) {
        setOpen(false);
      }
    },
    [includeTime, onChange, selectedDate],
  );

  const handleClear = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      setSelectedDate(undefined);
      onChange(undefined);
      setOpen(false);
    },
    [onChange],
  );

  const handleYearChange = useCallback((year: number) => {
    setViewDate((prev) => setYear(prev, year));
  }, []);

  const handleMonthChange = useCallback((month: number) => {
    setViewDate((prev) => setMonth(prev, month));
  }, []);

  const goToPrevMonth = useCallback(() => {
    setViewDate((prev) => setMonth(prev, getMonth(prev) - 1));
  }, []);

  const goToNextMonth = useCallback(() => {
    setViewDate((prev) => setMonth(prev, getMonth(prev) + 1));
  }, []);

  const displayText = useMemo(() => {
    if (!selectedDate) return placeholder || (includeTime ? 'YYYY-MM-DD HH:mm' : 'YYYY-MM-DD');
    return format(selectedDate, includeTime ? 'yyyy-MM-dd HH:mm' : 'yyyy-MM-dd');
  }, [selectedDate, includeTime, placeholder]);

  const calendarDays = useMemo(() => {
    const start = startOfWeek(startOfMonth(viewDate), { weekStartsOn: 0 });
    const end = endOfWeek(endOfMonth(viewDate), { weekStartsOn: 0 });
    return eachDayOfInterval({ start, end });
  }, [viewDate]);

  const currentYear = getYear(viewDate);
  const yearOptions = useMemo(() => {
    const years: number[] = [];
    for (let y = currentYear - 5; y <= currentYear + 5; y++) {
      years.push(y);
    }
    return years;
  }, [currentYear]);

  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [open]);

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
    <div className={styles.datePicker} ref={containerRef}>
      <button
        type="button"
        className={styles.trigger}
        onClick={() => !disabled && setOpen((v) => !v)}
        disabled={disabled}
        aria-haspopup="dialog"
        aria-expanded={open}
      >
        <Calendar
          size={14}
          style={{ marginRight: 8, verticalAlign: 'middle', color: 'var(--text-tertiary)' }}
        />
        <span className={selectedDate ? undefined : styles.placeholder}>{displayText}</span>
      </button>
      {selectedDate && !disabled && (
        <button
          type="button"
          className={styles.clearButton}
          onClick={handleClear}
          aria-label="清除"
          title="清除"
        >
          <X size={14} />
        </button>
      )}

      {open && (
        <div className={styles.popover} role="dialog" aria-label="日期选择">
          <div className={styles.header}>
            <button
              type="button"
              className={styles.navButton}
              onClick={goToPrevMonth}
              aria-label="上个月"
            >
              ‹
            </button>
            <div className={styles.selects}>
              <select
                className={styles.select}
                value={getYear(viewDate)}
                onChange={(e) => handleYearChange(Number(e.target.value))}
                aria-label="年份"
              >
                {yearOptions.map((y) => (
                  <option key={y} value={y}>
                    {y}
                  </option>
                ))}
              </select>
              <select
                className={styles.select}
                value={getMonth(viewDate)}
                onChange={(e) => handleMonthChange(Number(e.target.value))}
                aria-label="月份"
              >
                {Array.from({ length: 12 }, (_, i) => (
                  <option key={i} value={i}>
                    {format(setMonth(new Date(2020, 0, 1), i), 'MMM', { locale })}
                  </option>
                ))}
              </select>
            </div>
            <button
              type="button"
              className={styles.navButton}
              onClick={goToNextMonth}
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
                  onClick={() => handleSelect(day)}
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
                  setSelectedDate(next);
                  onChange(format(next, "yyyy-MM-dd'T'HH:mm"));
                }}
                aria-label="小时"
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
                  setSelectedDate(next);
                  onChange(format(next, "yyyy-MM-dd'T'HH:mm"));
                }}
                aria-label="分钟"
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
