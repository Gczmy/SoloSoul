import { useState, useRef, useEffect, useCallback, useMemo, Fragment } from 'react';
import { useTranslation } from 'react-i18next';
import {
  format,
  getMonth,
  getHours,
  getMinutes,
  setYear,
  setMonth,
  setHours,
  setMinutes,
} from 'date-fns';
import { zhCN, enUS } from 'date-fns/locale';
import { X } from 'lucide-react';
import styles from './DatePicker.module.css';
import { ICON_SIZE } from '@/lib/constants';
import { DatePickerCalendar } from './DatePickerCalendar';
import {
  SEGMENTS_CONFIG,
  SEPARATOR_BEFORE,
  EMPTY_SEGMENTS,
  type DatePickerProps,
  type SegmentKey,
  type Segments,
  parseValue,
  segmentsFromValue,
  buildValue,
  isSegmentsComplete,
  hasSegmentContent,
  draftString,
} from './DatePicker.helpers';

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
        <DatePickerCalendar
          viewDate={viewDate}
          selectedDate={selectedDate}
          includeTime={includeTime}
          locale={locale}
          onSelect={handleSelect}
          onApplyValue={(v) => applyValue(v)}
          onPrevMonth={goToPrevMonth}
          onNextMonth={goToNextMonth}
          onYearChange={handleYearChange}
          onMonthChange={handleMonthChange}
        />
      )}
    </div>
  );
}
