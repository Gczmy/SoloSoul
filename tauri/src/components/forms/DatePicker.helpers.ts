/**
 * P008：DatePicker 纯函数与类型（自 DatePicker.tsx 拆出，逐字节保持原实现）。
 * 分段配置、值↔分段互转、完整性校验等无状态逻辑；组件本体见 DatePicker.tsx。
 */
import { parseISO, isValid, getYear, getMonth, getDate, getHours, getMinutes } from 'date-fns';

export interface DatePickerProps {
  value?: string;
  onChange: (value: string | undefined) => void;
  includeTime?: boolean;
  disabled?: boolean;
}

/** 年月日时分 各分段配置：位数上限、空态占位（[y][y][y][y]-[m][m]-[d][d]）与无障碍标签 */
export const SEGMENTS_CONFIG = {
  year: { max: 4, placeholder: 'yyyy', label: '年份输入' },
  month: { max: 2, placeholder: 'mm', label: '月份输入' },
  day: { max: 2, placeholder: 'dd', label: '日期输入' },
  hour: { max: 2, placeholder: 'HH', label: '小时输入' },
  minute: { max: 2, placeholder: 'MM', label: '分钟输入' },
} as const;

export type SegmentKey = keyof typeof SEGMENTS_CONFIG;

export interface Segments {
  year: string;
  month: string;
  day: string;
  hour: string;
  minute: string;
}

export const EMPTY_SEGMENTS: Segments = { year: '', month: '', day: '', hour: '', minute: '' };

/** 分段前的分隔符：yyyy-mm-dd HH:MM */
export const SEPARATOR_BEFORE: Partial<Record<SegmentKey, string>> = {
  month: '-',
  day: '-',
  hour: ' ',
  minute: ':',
};

export function parseValue(value?: string): Date | undefined {
  if (!value) return undefined;
  const d = parseISO(value);
  return isValid(d) ? d : undefined;
}

/** 从受控值解析各分段字符串（空值/非法 → 全空） */
export function segmentsFromValue(value?: string): Segments {
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
export function pad2(v: string): string {
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
export function buildValue(s: Segments, includeTime: boolean | undefined): string | null {
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
export function isSegmentsComplete(s: Segments, includeTime: boolean | undefined): boolean {
  if (s.year.length !== 4 || !s.month || !s.day) return false;
  if (includeTime && (!s.hour || !s.minute)) return false;
  return true;
}

/** 当前模式下是否有任意分段内容（用于「填了但未输满」的静默丢值防护） */
export function hasSegmentContent(s: Segments, includeTime: boolean | undefined): boolean {
  if (s.year || s.month || s.day) return true;
  if (includeTime && (s.hour || s.minute)) return true;
  return false;
}

/** 由分段拼出草稿字符串（不校验合法性，供非法/未输满草稿提交给父组件做保存校验）；
 *  单数字同样补零；去掉尾部空分段（仅填年份 → '2024'，填了年月 → '2024-12'）。 */
export function draftString(s: Segments, includeTime: boolean | undefined): string {
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
