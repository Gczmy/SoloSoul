import { parseISO, isValid } from 'date-fns';
import type { PropertyType } from '@/types/template';

export interface FieldValidator {
  isValid: (v: string) => boolean;
  hintKey: string;
}

/**
 * 对象字段类型值校验（对象编辑页保存校验 + 动态组子字段校验共用）。
 * 日期/日期时间与 DatePicker 内部解析保持一致（date-fns parseISO + isValid）：
 * 仅接受严格 ISO 格式，拒绝不可能日期（2024-02-30、2024-13-01、24:00 等）。
 * 不用 Date.parse —— 它会对 "2024/12/31"、"December 31, 2024" 等宽松格式放行，
 * 而这些格式 DatePicker 永远不会产生，放行只会掩盖脏数据。
 */
export const FIELD_TYPE_VALIDATORS: Partial<Record<PropertyType, FieldValidator>> = {
  email: {
    isValid: (v) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v),
    hintKey: 'editor:validation_email',
  },
  phone: {
    isValid: (v) => /^[+\d][\d\s()-]{3,}$/.test(v),
    hintKey: 'editor:validation_phone',
  },
  url: {
    isValid: (v) => /^https?:\/\/.+\.+/.test(v),
    hintKey: 'editor:validation_url',
  },
  number: {
    isValid: (v) => !Number.isNaN(Number(v)),
    hintKey: 'editor:validation_number',
  },
  date: {
    isValid: (v) => isValid(parseISO(v)),
    hintKey: 'editor:validation_date',
  },
  datetime: {
    // 与 DatePicker 显示/解析能力对齐（date-fns parseISO 接受 'T' 或空格分隔、
    // 可含秒/时区）：先收紧为日期时间形态（HH 00-23 / MM 00-59，parseISO 会把
    // 24:00 归一到次日故必须先行收紧），再让 parseISO 拒绝不可能日期（2024-02-30）。
    isValid: (v) => {
      const m =
        /^(\d{4}-\d{2}-\d{2})[T ]([01]\d|2[0-3]):([0-5]\d)(?::[0-5]\d)?(?:Z|[+-]\d{2}:?\d{2})?$/.exec(
          v,
        );
      return !!m && isValid(parseISO(v));
    },
    hintKey: 'editor:validation_datetime',
  },
};

export function isFieldValueValid(type: PropertyType, value: string): boolean {
  const validator = FIELD_TYPE_VALIDATORS[type];
  return validator ? validator.isValid(value) : true;
}
