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
import { X } from 'lucide-react';
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
  if (d) {
    return {
      year: String(getYear(d)).padStart(4, '0'),
      month: String(getMonth(d) + 1).padStart(2, '0'),
      day: String(getDate(d)).padStart(2, '0'),
      hour: String(getHours(d)).padStart(2, '0'),
      minute: String(getMinutes(d)).padStart(2, '0'),
    };
  }
  if (value) {
    // 宽松解析：ISO 形态但不可能日期（2024-02-30T10:30 等存量脏数据）也尽力填充
    // 分段，让字段显示原始值而非「看似为空」——保存报错时用户能看到并修正/清空。
    const m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2}))?/.exec(value);
    if (m) {
      return {
        year: m[1],
        month: m[2],
        day: m[3],
        hour: m[4] || '',
        minute: m[5] || '',
      };
    }
  }
  return { ...EMPTY_SEGMENTS };
}

/** 单数字补零（1 → 01）；空/两位及以上原样返回。 */
function pad2(v: string): string {
  return v.length === 1 ? `0${v}` : v;
}

/**
 * 由分段拼装提交值。
 * - 全部为空 → ''（表示清除）
 * - 未输满/非法（如 13 月、2 月 30 日、时 25）→ null（暂不提交，等待继续输入）
 * - 完整合法 → 'yyyy-MM-dd' 或 'yyyy-MM-ddTHH:mm'（includeTime 时必须年月日时分齐全）
 *
 * 单数字月/日/时/分自动补零（1 → 01）：否则用户输入 `2024-1-5` 这类
 * 常见写法时值永不提交、保存静默丢弃（字段看着填了，库里却没有）。
 * 补零后产出的是合法 ISO（`2024-01-05`、`T01:05`），不会触发解析回环。
 */
function buildValue(s: Segments, includeTime: boolean | undefined): string | null {
  const { year, month, day, hour, minute } = s;
  if (!year && !month && !day && !hour && !minute) return '';
  if (year.length !== 4) return null;
  const m = pad2(month);
  const d = pad2(day);
  if (m.length !== 2 || d.length !== 2) return null;
  const y = Number(year);
  const mNum = Number(m);
  const dNum = Number(d);
  if (mNum < 1 || mNum > 12) return null;
  const date = new Date(y, mNum - 1, dNum);
  // 回环校验：2024-02-30 会被 Date 归一为 2024-03-01，必须拒绝
  if (date.getFullYear() !== y || date.getMonth() !== mNum - 1 || date.getDate() !== dNum) {
    return null;
  }
  if (!includeTime) return `${year}-${m}-${d}`;
  const h = pad2(hour);
  const min = pad2(minute);
  if (h.length !== 2 || min.length !== 2) return null;
  if (Number(h) > 23 || Number(min) > 59) return null;
  return `${year}-${m}-${d}T${h}:${min}`;
}

/** 当前模式下所有分段是否都已输满（年月日；includeTime 时含时分；单数字视为已输满） */
function isSegmentsComplete(s: Segments, includeTime: boolean | undefined): boolean {
  if (s.year.length !== 4 || !s.month || !s.day) return false;
  if (includeTime && (!s.hour || !s.minute)) return false;
  return true;
}

/** 当前模式下是否有任意分段内容（用于「填了但未输满」的静默丢值防护） */
function hasSegmentContent(s: Segments, includeTime: boolean | undefined): boolean {
  if (s.year || s.month || s.day) return true;
  if (includeTime && (s.hour || s.minute)) return true;
  return false;
}

/** 由分段拼出草稿字符串（不校验合法性，供非法/未输满草稿提交给父组件做保存校验）；
 *  单数字同样补零；去掉尾部空分段（仅填年份 → '2024'，填了年月 → '2024-12'）。 */
function draftString(s: Segments, includeTime: boolean | undefined): string {
  const dateSegs = [s.year, pad2(s.month), pad2(s.day)];
  while (dateSegs.length > 0 && dateSegs[dateSegs.length - 1] === '') dateSegs.pop();
  let out = dateSegs.join('-');
  if (includeTime) {
    const timeSegs = [pad2(s.hour), pad2(s.minute)];
    while (timeSegs.length > 0 && timeSegs[timeSegs.length - 1] === '') timeSegs.pop();
    if (timeSegs.length > 0) out += `T${timeSegs.join(':')}`;
  }
  return out;
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
  // 最近一次合法提交值（非法草稿传播后，分段被改回未输满时用于撤销草稿、恢复旧值）
  const lastValidRef = useRef<string | undefined>(parseValue(value) ? value : undefined);
  // 当前父值是否需要「撤销恢复」：来自非法草稿传播（propagateDraft），或
  // 存量值本身无法解析（DatePicker 显示为宽松分段，清空时应一并清除）。
  const propagatedDraftRef = useRef(!!value && !parseValue(value));
  // 标记：内部传播草稿期间禁止 effect 重置分段显示，避免 `1212-1` 被 effect 同步为 `1212-01`。
  const skipEffectSyncRef = useRef(false);

  const segmentKeys: SegmentKey[] = useMemo(
    () => (includeTime ? ['year', 'month', 'day', 'hour', 'minute'] : ['year', 'month', 'day']),
    [includeTime],
  );

  useEffect(() => {
    const d = parseValue(value);
    setSelectedDate(d);
    if (d) setViewDate(d);
    // 内部传播草稿期间跳过分段同步：分段保持用户输入原样（如 month='1'），
    // 不被 effect 重置为补零值（如 month='01'），避免单数字输入时自动补零干扰连续输入。
    if (skipEffectSyncRef.current) {
      skipEffectSyncRef.current = false;
      return;
    }
    // 外部（日历选择/清除/回退）改变 value 时同步分段草稿
    if (value !== lastCommittedRef.current) {
      setSegments(segmentsFromValue(value));
      lastCommittedRef.current = value;
      propagatedDraftRef.current = !!value && !parseValue(value);
    }
  }, [value]);

  /** 统一提交入口：同步 lastCommitted、选中态并通知父组件。
   *  @param preserveSegments true — 内部从分段提交（如用户键入），保留分段原样不归一化；
   *                         false/省略 — 外部来源（日历选择/清除/载入），同步分段为解析后的值。 */
  const applyValue = useCallback(
    (v: string | undefined, preserveSegments?: boolean) => {
      lastCommittedRef.current = v;
      const d = parseValue(v);
      if (d) lastValidRef.current = v;
      propagatedDraftRef.current = false;
      skipEffectSyncRef.current = true;
      setSelectedDate(d);
      // 内部提交：保留用户输入的原始分段（如 day='5' 而非被归一为 '05'），
      // 连续输入时不会出现「输入 5 立刻变成 05」的干扰。
      if (!preserveSegments) {
        setSegments(segmentsFromValue(v));
      }
      onChange(v);
    },
    [onChange],
  );

  /** 提交非法完整草稿：仅同步 lastCommitted（防外部 value 回写覆盖分段），
   *  不改选中态/分段显示——草稿仍留在输入框内，保存时由父组件校验并在字段下报错。
   *  置位 propagatedDraftRef：随后删改回未输满时走撤销恢复（round-2 语义）。 */
  const propagateDraft = useCallback(
    (draft: string, markForRevert: boolean) => {
      lastCommittedRef.current = draft;
      if (markForRevert) propagatedDraftRef.current = true;
      skipEffectSyncRef.current = true; // 草稿传播不重置分段
      onChange(draft);
    },
    [onChange],
  );

  /** 撤销非法草稿：恢复为最近一次合法提交值（或空），分段保持原状。 */
  const restoreValue = useCallback(
    (v: string | undefined) => {
      lastCommittedRef.current = v;
      propagatedDraftRef.current = false;
      skipEffectSyncRef.current = true; // 恢复值不重置分段
      onChange(v);
    },
    [onChange],
  );

  const commitSegments = useCallback(
    (next: Segments) => {
      const committed = buildValue(next, includeTime);
      if (committed === null) {
        if (isSegmentsComplete(next, includeTime)) {
          // 完整但非法（2024-02-30 / 2024-13-01 / 25:00 等）：草稿提交给父组件，
          // 保存时校验可在对应字段下报错；置位撤销标记供删改时恢复。
          propagateDraft(draftString(next, includeTime), true);
        } else if (propagatedDraftRef.current) {
          // 非法完整草稿被改回未输满（如删掉某一段）：撤销草稿，恢复最近一次合法值（或空），
          // 避免保存时对「看似未填写」的字段误报错。
          restoreValue(lastValidRef.current);
        } else if (hasSegmentContent(next, includeTime)) {
          // 有内容但未输满（年份不足 4 位如 `12-12-12`、或部分分段缺失）：
          // 同样传播草稿（不置撤销标记）——否则保存时值不进 values、校验跳过
          // 空字段，对象以空属性落库，表现为「保存成功但字段没了」。
          // 传播后保存校验会识别该值非法并在对应字段下报错，用户能看见并修正。
          propagateDraft(draftString(next, includeTime), false);
        }
        return;
      }
      if (committed === '') {
        if (lastCommittedRef.current !== undefined) {
          applyValue(undefined);
        }
        return;
      }
      if (committed !== lastCommittedRef.current) {
        applyValue(committed, true); // 保留用户输入原样（如 day='5'），不归一为 '05'
      }
    },
    [includeTime, applyValue, propagateDraft, restoreValue],
  );

  const focusSegment = useCallback((key: SegmentKey) => {
    segmentRefs.current[key]?.focus();
  }, []);

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
      {/* 点击整个输入区域（含分段/分隔符/留白）弹出日历卡片；clear 按钮除外 */}
      <div className={styles.triggerRow} role="group" onClick={() => !disabled && setOpen(true)}>
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
                  // 点击输入框同时弹出日历卡片（clear 按钮除外——它不是分段输入）
                  setOpen(true);
                  // 聚焦即全选：用户直接输入数字即可覆盖原有内容
                  e.currentTarget.select();
                }}
                onBlur={() => setFocusedKey(null)}
              />
            </Fragment>
          );
        })}
        {/* 清除按钮置于最右侧：handleClear 内已 stopPropagation，不触发行级弹出 */}
        {hasValue && !disabled && (
          <button
            type="button"
            className={[styles.iconButton, styles.clearAction].join(' ')}
            onClick={handleClear}
            aria-label="清除"
            title="清除"
          >
            <X size={ICON_SIZE.sm} />
          </button>
        )}
      </div>
      {focusedKey && !disabled && (
        <div className={styles.inputHint}>可直接输入数字；也可在日历中选择日期</div>
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
