import { describe, it, expect, vi } from 'vitest';
import {
  normalizeFieldKey,
  conflictFieldLabel,
  nestedFieldLabel,
  formatConflictValue,
  truncateConflictValue,
} from './conflictFieldMeta';

/** 模拟 i18n：settings:/editor: 命中返回假想译文，未命中返回 defaultValue。 */
function makeT(overrides: Record<string, string> = {}) {
  return vi.fn((key: string, opts?: { defaultValue?: string }) => {
    return overrides[key] ?? opts?.defaultValue ?? key;
  });
}

describe('normalizeFieldKey', () => {
  it('converts camelCase to snake_case', () => {
    expect(normalizeFieldKey('childrenIds')).toBe('children_ids');
    expect(normalizeFieldKey('accountId')).toBe('account_id');
    expect(normalizeFieldKey('isDeleted')).toBe('is_deleted');
  });

  it('keeps snake_case as-is', () => {
    expect(normalizeFieldKey('account_id')).toBe('account_id');
  });
});

describe('conflictFieldLabel', () => {
  it('uses locale label for known fields', () => {
    const t = makeT({ 'settings:sync_conflict_field_template_id': '模板' });
    expect(conflictFieldLabel('templateId', t)).toBe('模板');
    // 兼容 snake_case 输入
    expect(conflictFieldLabel('template_id', t)).toBe('模板');
  });

  it('humanizes unknown fields as fallback', () => {
    const t = makeT();
    expect(conflictFieldLabel('someUnknownField', t)).toBe('Some Unknown Field');
  });
});

describe('nestedFieldLabel', () => {
  it('uses editor:fields label for dynamic property ids', () => {
    const t = makeT({ 'editor:fields.fullName': '姓名' });
    expect(nestedFieldLabel('fullName', t)).toBe('姓名');
  });

  it('humanizes unknown property ids', () => {
    const t = makeT();
    expect(nestedFieldLabel('customField', t)).toBe('Custom Field');
  });
});

describe('formatConflictValue', () => {
  it('maps seeded template ids to i18n names', () => {
    const t = makeT({ 'settings:sync_conflict_tpl_passport': '护照' });
    expect(formatConflictValue('templateId', 'passport', t)).toBe('护照');
    // 未知模板 id 保持原值
    expect(formatConflictValue('templateId', 'tpl-9f3a', t)).toBe('tpl-9f3a');
  });

  it('maps section/category/type codes to i18n names', () => {
    const t = makeT({
      'settings:sync_conflict_cat_travel': '旅行',
      'settings:sync_conflict_type_note': '笔记',
    });
    expect(formatConflictValue('sectionType', 'travel', t)).toBe('旅行');
    expect(formatConflictValue('typeId', 'note', t)).toBe('笔记');
  });

  it('localizes booleans', () => {
    const t = makeT({
      'settings:sync_conflict_value_true': '是',
      'settings:sync_conflict_value_false': '否',
    });
    expect(formatConflictValue('isDeleted', true, t)).toBe('是');
    expect(formatConflictValue('isDeleted', false, t)).toBe('否');
  });

  it('keeps plain scalars as-is', () => {
    const t = makeT();
    expect(formatConflictValue('name', '张三', t)).toBe('张三');
    expect(formatConflictValue('version', 3, t)).toBe('3');
  });

  it('formats nested objects as readable lines with field labels', () => {
    const t = makeT({
      'editor:fields.fullName': '姓名',
      'editor:fields.email': '邮箱',
    });
    const out = formatConflictValue('properties', { fullName: '张三', email: 'a@b.com' }, t);
    expect(out).toContain('姓名: 张三');
    expect(out).toContain('邮箱: a@b.com');
  });

  it('formats arrays as bullet lines', () => {
    const t = makeT();
    const out = formatConflictValue('childrenIds', ['obj-1', 'obj-2'], t);
    expect(out).toBe('- obj-1\n- obj-2');
  });
});

describe('truncateConflictValue', () => {
  it('keeps short text and truncates long text', () => {
    expect(truncateConflictValue('abc')).toBe('abc');
    const long = 'x'.repeat(700);
    const out = truncateConflictValue(long);
    expect(out.length).toBeLessThan(long.length);
    expect(out.endsWith('…')).toBe(true);
  });
});
