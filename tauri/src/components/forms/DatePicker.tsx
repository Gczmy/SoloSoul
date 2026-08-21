import { useState, useRef, useEffect, useCallback, useMemo, Fragment } from 'react';
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
  getDate,
  getHours,
  getMinutes,
  setYear,
  setMonth,
  setHours,
  setMinutes,
} from 'date-fns';
import { zhCN, enUS } from 'date-fns/locale';
import { Calendar, X } from 'lucide-react';
import { DropdownSelect } from '@/components/ui/DropdownSelect';
import styles from './DatePicker.module.css';
import { ICON_SIZE } from '@/lib/constants';

interface DatePickerProps {
  value?: string;
  onChange: (value: string | undefined) => void;
  includeTime?: boolean;
  disabled?: boolean;
}

/** 年月日时分 各分段配置：位数上限、空态占位（[y][y][y][y]-[m][m]-[d][d]）与无障碍标签 */
const SEGMENTS_CONFIG = {
  year: { max: 4, placeholder: 'yyyy', label: '年份输入' },
  month: { max: 2, placeholder: 'mm', label: '月份输入' },
  day: { max: 2, placeholder: 'dd', label: '日期输入' },
  hour: { max: 2, placeholder: 'HH', label: '小时输入' },
  minute: { max: 2, placeholder: 'MM', label: '分钟输入' },
} as const;

type SegmentKey = keyof typeof SEGMENTS_CONFIG;

interface Segments {
  year: string;
  month: string;
  day: string;
  hour: string;
  minute: string;
}

const EMPTY_SEGMENTS: Segments = { year: '', month: '', day: '', hour: '', minute: '' };

/** 分段前的分隔符：yyyy-mm-dd HH:MM */
const SEPARATOR_BEFORE: Partial<Record<SegmentKey, string>> = {
  month: '-',
  day: '-',
  hour: ' ',
  minute: ':',
};

function parseValue(value?: string): Date | undefined {
  if (!value) return undefined;
  const d = parseISO(value);
  return isValid(d) ? d : undefined;
}

/** 从受控值解析各分段字符串（空值/非法 → 全空） */
function segmentsFromValue(value?: string): Segments {
  const d = parseValue(value);
  if (!d) return { ...EMPTY_SEGMENTS };
  return {
    year: String(getYear(d)).padStart(4, '0'),
    month: String(getMonth(d) + 1).padStart(2, '0'),
    day: String(getDate(d)).padStart(2, '0'),
    hour: String(getHours(d)).padStart(2, '0'),
    minute: String(getMinutes(d)).padStart(2, '0'),
  };
}

/**
 * 由分段拼装提交值。
 * - 全部为空 → ''（表示清除）
 * - 未输满/非法（如 13 月、2 月 30 日、时 25）→ null（暂不提交，等待继续输入）
 * - 完整合法 → 'yyyy-MM-dd' 或 'yyyy-MM-ddTHH:mm'（includeTime 时必须年月日时分齐全）
 */
function buildValue(s: Segments, includeTime: boolean | undefined): string | null {
  const { year, month, day, hour, minute } = s;
  if (!year && !month && !day && !hour && !minute) return '';
  if (year.length !== 4 || month.length !== 2 || day.length !== 2) return null;
  const y = Number(year);
  const m = Number(month);
  const d = Number(day);
  if (m < 1 || m > 12) return null;
  const date = new Date(y, m - 1, d);
  // 回环校验：2024-02-30 会被 Date 归一为 2024-03-01，必须拒绝
  if (date.getFullYear() !== y || date.getMonth() !== m - 1 || date.getDate() !== d) {
    return null;
  }
  if (!includeTime) return `${year}-${month}-${day}`;
  if (!hour || !minute) return null;
  if (Number(hour) > 23 || Number(minute) > 59) return null;
  return `${year}-${month}-${day}T${hour}:${minute}`;
}

export function DatePicker({ value, onChange, includeTime, disabled }: DatePickerProps) {
  const { i18n } = useTranslation();
  const locale = useMemo(() => (i18n.language.startsWith('zh') ? zhCN : enUS), [i18n.language]);

  const [open, setOpen] = useState(false);
  const [viewDate, setViewDate] = useState<Date>(() => parseValue(value) || new Date());
  const [selectedDate, setSelectedDate] = useState<Date | undefined>(() => parseValue(value));
  const [segments, setSegments] = useState<Segments>(() => segmentsFromValue(value));
  const [focusedKey, setFocusedKey] = useState<SegmentKey | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const segmentRefs = useRef<Partial<Record<SegmentKey, HTMLInputElement | null>>>({});
  // 最近一次已提交给父组件的值：外部 value 变化时据此同步分段，避免自身输入被覆盖
  const lastCommittedRef = useRef<string | undefined>(value);

  const segmentKeys: SegmentKey[] = useMemo(
    () => (includeTime ? ['year', 'month', 'day', 'hour', 'minute'] : ['year', 'month', 'day']),
    [includeTime],
  );

  useEffect(() => {
    const d = parseValue(value);
    setSelectedDate(d);
    if (d) setViewDate(d);
    // 外部（日历选择/清除/回退）改变 value 时同步分段草稿
    if (value !== lastCommittedRef.current) {
      setSegments(segmentsFromValue(value));
      lastCommittedRef.current = value;
    }
  }, [value]);

  /** 统一提交入口：同步 lastCommitted、选中态、分段显示并通知父组件。 */
  const applyValue = useCallback(
    (v: string | undefined) => {
      lastCommittedRef.current = v;
      setSelectedDate(parseValue(v));
      setSegments(segmentsFromValue(v));
      onChange(v);
    },
    [onChange],
  );

  const commitSegments = useCallback(
    (next: Segments) => {
      const committed = buildValue(next, includeTime);
      if (committed === null) return; // 未输满/非法：等待继续输入
      if (committed === '') {
        if (lastCommittedRef.current !== undefined) {
          applyValue(undefined);
        }
        return;
      }
      if (committed !== lastCommittedRef.current) {
        applyValue(committed);
      }
    },
    [includeTime, applyValue],
  );

  const focusSegment = useCallback(
    (key: SegmentKey) => {
      segmentRefs.current[key]?.focus();
    },
    [],
  );

  const handleSegmentChange = useCallback(
    (key: SegmentKey, raw: string) => {
      const digits = raw.replace(/\D/g, '').slice(0, SEGMENTS_CONFIG[key].max);
      const next = { ...segments, [key]: digits };
      setSegments(next);
      // 输满自动跳到下一段
      if (digits.length === SEGMENTS_CONFIG[key].max) {
        const idx = segmentKeys.indexOf(key);
        if (idx >= 0 && idx < segmentKeys.length - 1) {
          focusSegment(segmentKeys[idx + 1]);
        }
      }
      commitSegments(next);
    },
    [segments, segmentKeys, focusSegment, commitSegments],
  );

  const isSegmentInvalid = useCallback(
    (key: SegmentKey): boolean => {
      const v = segments[key];
      if (v.length !== SEGMENTS_CONFIG[key].max) return false;
      switch (key) {
        case 'month': {
          const m = Number(v);
          return m < 1 || m > 12;
        }
        case 'day': {
          const d = Number(v);
          if (d < 1 || d > 31) return true;
          // 年月完整时做真实日期校验（如 2024-02-30）
          if (segments.year.length === 4 && segments.month.length === 2) {
            const date = new Date(Number(segments.year), Number(segments.month) - 1, d);
            return (
              date.getFullYear() !== Number(segments.year) ||
              date.getMonth() !== Number(segments.month) - 1 ||
              date.getDate() !== d
            );
          }
          return false;
        }
        case 'hour':
          return Number(v) > 23;
        case 'minute':
          return Number(v) > 59;
        default:
          return false;
      }
    },
    [segments],
  );

  const handleSelect = useCallback(
    (day: Date) => {
      let next = day;
      if (selectedDate && includeTime) {
        next = setHours(setMinutes(day, selectedDate.getMinutes()), selectedDate.getHours());
      }
      applyValue(format(next, includeTime ? "yyyy-MM-dd'T'HH:mm" : 'yyyy-MM-dd'));
      if (!includeTime) {
        setOpen(false);
      }
    },
    [includeTime, applyValue, selectedDate],
  );

  const handleClear = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      applyValue(undefined);
      setOpen(false);
    },
    [applyValue],
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

  const hasValue = !!selectedDate;

  return (
    <div className={styles.datePicker} ref={containerRef}>
      <div className={styles.triggerRow} role="group">
        {segmentKeys.map((key) => {
          const config = SEGMENTS_CONFIG[key];
          const invalid = isSegmentInvalid(key);
          return (
            <Fragment key={key}>
              {SEPARATOR_BEFORE[key] && (
                <span className={styles.separator} aria-hidden="true">
                  {SEPARATOR_BEFORE[key]}
                </span>
              )}
              <input
                ref={(el) => {
                  segmentRefs.current[key] = el;
                }}
                type="text"
                inputMode="numeric"
                autoComplete="off"
                maxLength={config.max}
                className={[
                  styles.segment,
                  key === 'year' && styles.yearSegment,
                  focusedKey === key && styles.segmentEditing,
                  invalid && styles.segmentInvalid,
                ]
                  .filter(Boolean)
                  .join(' ')}
                value={segments[key]}
                placeholder={config.placeholder}
                aria-label={config.label}
                aria-invalid={invalid || undefined}
                title="点击后可直接输入数字覆盖"
                disabled={disabled}
                onChange={(e) => handleSegmentChange(key, e.target.value)}
                onKeyDown={(e) => {
                  // 空段退格回到上一段，便于快速修正
                  if (e.key === 'Backspace' && !e.currentTarget.value) {
                    e.preventDefault();
                    const idx = segmentKeys.indexOf(key);
                    if (idx > 0) focusSegment(segmentKeys[idx - 1]);
                  }
                }}
                onFocus={(e) => {
                  setFocusedKey(key);
                  // 聚焦即全选：用户直接输入数字即可覆盖原有内容
                  e.currentTarget.select();
                }}
                onBlur={() => setFocusedKey(null)}
              />
            </Fragment>
          );
        })}
        <div className={styles.actions}>
          {hasValue && !disabled && (
            <button
              type="button"
              className={styles.iconButton}
              onClick={handleClear}
              aria-label="清除"
              title="清除"
            >
              <X size={ICON_SIZE.sm} />
            </button>
          )}
          <button
            type="button"
            className={styles.iconButton}
            onClick={() => !disabled && setOpen((v) => !v)}
            disabled={disabled}
            aria-haspopup="dialog"
            aria-expanded={open}
            aria-label="打开日历"
            title="打开日历选择"
          >
            <Calendar
              size={ICON_SIZE.sm}
              style={{ color: 'var(--text-tertiary)' }}
            />
          </button>
        </div>
      </div>
      {focusedKey && !disabled && (
        <div className={styles.inputHint}>可直接输入数字；也可点击右侧日历选择日期</div>
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
            </button>{' '}
            <div className={styles.selects}>
              <DropdownSelect
                value={getYear(viewDate)}
                onChange={(v) => handleYearChange(Number(v))}
                options={yearOptions.map((y) => ({ value: y, label: String(y) }))}
                triggerLabel={String(getYear(viewDate))}
                ariaLabel="选择年份"
              />
              <DropdownSelect
                value={getMonth(viewDate)}
                onChange={(v) => handleMonthChange(Number(v))}
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
                  applyValue(format(next, "yyyy-MM-dd'T'HH:mm"));
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
                  applyValue(format(next, "yyyy-MM-dd'T'HH:mm"));
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
