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
import { Calendar, X, ChevronDown } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
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
  const [yearOpen, setYearOpen] = useState(false);
  const [monthOpen, setMonthOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const yearListRef = useRef<HTMLDivElement>(null);
  const monthListRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const d = parseValue(value);
    setSelectedDate(d);
    if (d) setViewDate(d);
  }, [value]);

  // Auto-scroll year list to current year when opened
  useEffect(() => {
    if (yearOpen && yearListRef.current) {
      const currentYear = getYear(viewDate);
      const item = yearListRef.current.querySelector(`[data-year="${currentYear}"]`);
      if (item) {
        item.scrollIntoView({ block: 'center' });
      }
    }
  }, [yearOpen, viewDate]);

  // Auto-scroll month list to current month when opened
  useEffect(() => {
    if (monthOpen && monthListRef.current) {
      const currentMonth = getMonth(viewDate);
      const item = monthListRef.current.querySelector(`[data-month="${currentMonth}"]`);
      if (item) {
        item.scrollIntoView({ block: 'center' });
      }
    }
  }, [monthOpen, viewDate]);

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
    setYearOpen(false);
  }, []);

  const handleMonthChange = useCallback((month: number) => {
    setViewDate((prev) => setMonth(prev, month));
    setMonthOpen(false);
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
    // Reasonable range for native select: covers 1900s to future
    for (let y = currentYear - 80; y <= currentYear + 10; y++) {
      years.push(y);
    }
    return years;
  }, [currentYear]);

  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setYearOpen(false);
        setMonthOpen(false);
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
            </button>                <div className={styles.selects} onMouseDown={(e) => {
                // Close popups when clicking outside their area
                const yearArea = e.currentTarget.querySelector('[data-year-area]');
                if (yearArea && !yearArea.contains(e.target as Node)) {
                  setYearOpen(false);
                }
                const monthArea = e.currentTarget.querySelector('[data-month-area]');
                if (monthArea && !monthArea.contains(e.target as Node)) {
                  setMonthOpen(false);
                }
              }}>
              {/* Custom scrollable year dropdown */}
              <div data-year-area style={{ position: 'relative' }}>
                <button
                  type="button"
                  onClick={() => setYearOpen((v) => !v)}
                  className={styles.dropdownTrigger}
                  aria-label="选择年份"
                >
                  {getYear(viewDate)}
                  <ChevronDown size={12} />
                </button>
                <AnimatePresence>
                  {yearOpen && (
                    <motion.div
                      className={styles.dropdownPopover}
                      ref={yearListRef}
                      initial={{ opacity: 0, y: -6, scale: 0.96 }}
                      animate={{ opacity: 1, y: 0, scale: 1 }}
                      exit={{ opacity: 0, y: -6, scale: 0.96 }}
                      transition={{ duration: 0.15, ease: 'easeOut' }}
                      style={{ transformOrigin: 'top' }}
                    >
                      {yearOptions.map((y) => (
                        <button
                          key={y}
                          type="button"
                          data-year={y}
                          className={`${styles.dropdownItem} ${y === getYear(viewDate) ? styles.dropdownItemActive : ''}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            handleYearChange(y);
                          }}
                        >
                          {y}
                        </button>
                      ))}
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
              <div data-month-area style={{ position: 'relative' }}>
                <button
                  type="button"
                  onClick={() => setMonthOpen((v) => !v)}
                  className={styles.dropdownTrigger}
                  aria-label="选择月份"
                >
                  {format(viewDate, 'MMM', { locale })}
                  <ChevronDown size={12} />
                </button>
                <AnimatePresence>
                  {monthOpen && (
                    <motion.div
                      className={styles.dropdownPopover}
                      ref={monthListRef}
                      initial={{ opacity: 0, y: -6, scale: 0.96 }}
                      animate={{ opacity: 1, y: 0, scale: 1 }}
                      exit={{ opacity: 0, y: -6, scale: 0.96 }}
                      transition={{ duration: 0.15, ease: 'easeOut' }}
                      style={{ transformOrigin: 'top' }}
                    >
                      {Array.from({ length: 12 }, (_, i) => (
                        <button
                          key={i}
                          type="button"
                          data-month={i}
                          className={`${styles.dropdownItem} ${i === getMonth(viewDate) ? styles.dropdownItemActive : ''}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            handleMonthChange(i);
                          }}
                        >
                          {format(setMonth(new Date(2020, 0, 1), i), 'MMM', { locale })}
                        </button>
                      ))}
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
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
