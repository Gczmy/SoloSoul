import { describe, it, expect, vi } from 'vitest';
import {
  normalizeFieldKey,
  conflictFieldLabel,
  nestedFieldLabel,
  formatConflictValue,
  truncateConflictValue,
  buildDiffEntries,
  isSensitivityLevel,
  resolveConflictIcon,
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

  it('localizes template_type values', () => {
    const t = makeT({ 'settings:sync_conflict_tpltype_user': '用户模板' });
    expect(formatConflictValue('templateType', 'user', t)).toBe('用户模板');
    // 未知模板类型保持原值
    expect(formatConflictValue('templateType', 'unknown_type', t)).toBe('unknown_type');
  });

  it('localizes identity type values', () => {
    const t = makeT({ 'settings:sync_conflict_cat_identity': '身份' });
    expect(formatConflictValue('typeId', 'identity', t)).toBe('身份');
    expect(formatConflictValue('typeId', 'note', t)).toBe('note');
  });

  it('localizes icon field values', () => {
    const t = makeT({ 'settings:sync_conflict_icon_document': '文档' });
    expect(formatConflictValue('iconName', 'document', t)).toBe('文档');
    // 未知图标保持原值
    expect(formatConflictValue('iconName', 'unknown_icon', t)).toBe('unknown_icon');
  });
});

describe('resolveConflictIcon', () => {
  it('resolves icon fields to lucide components', () => {
    expect(resolveConflictIcon('iconName', 'document')).not.toBeNull();
    expect(resolveConflictIcon('icon_id', 'credit_card')).not.toBeNull();
    expect(resolveConflictIcon('iconSnapshot', 'passport')).not.toBeNull();
  });

  it('returns null for non-icon fields or non-string values', () => {
    expect(resolveConflictIcon('name', 'document')).toBeNull();
    expect(resolveConflictIcon('iconName', 123)).toBeNull();
    expect(resolveConflictIcon('iconName', null)).toBeNull();
    expect(resolveConflictIcon('iconName', undefined)).toBeNull();
  });
});

describe('isSensitivityLevel', () => {
  it('recognizes the four sensitivity tokens', () => {
    expect(isSensitivityLevel('public')).toBe(true);
    expect(isSensitivityLevel('internal')).toBe(true);
    expect(isSensitivityLevel('sensitive')).toBe(true);
    expect(isSensitivityLevel('critical')).toBe(true);
  });

  it('rejects non-sensitivity values', () => {
    expect(isSensitivityLevel('internalx')).toBe(false);
    expect(isSensitivityLevel('Internal')).toBe(false);
    expect(isSensitivityLevel('张三')).toBe(false);
    expect(isSensitivityLevel(123)).toBe(false);
    expect(isSensitivityLevel(null)).toBe(false);
    expect(isSensitivityLevel(undefined)).toBe(false);
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

describe('buildDiffEntries', () => {
  it('returns null for scalar fields (plain rendering)', () => {
    const t = makeT();
    expect(buildDiffEntries('name', '张三', '张三', t)).toBeNull();
    expect(buildDiffEntries('version', 1, 2, t)).toBeNull();
  });

  it('returns null when both sides are empty containers', () => {
    const t = makeT();
    expect(buildDiffEntries('properties', {}, {}, t)).toBeNull();
    expect(buildDiffEntries('childrenIds', [], [], t)).toBeNull();
  });

  it('pairs object leaves and flags only the differing ones', () => {
    const t = makeT({ 'editor:fields.fullName': '姓名', 'editor:fields.email': '邮箱' });
    const entries = buildDiffEntries(
      'properties',
      { fullName: '张三', email: 'a@b.com' },
      { fullName: '张三', email: 'a@c.com' },
      t,
    );
    expect(entries).toHaveLength(2);
    const email = entries?.find((e) => e.label === '邮箱');
    expect(email?.localText).toBe('a@b.com');
    expect(email?.remoteText).toBe('a@c.com');
    expect(email?.changed).toBe(true);
    const name = entries?.find((e) => e.label === '姓名');
    expect(name?.localText).toBe('张三');
    expect(name?.changed).toBe(false);
  });

  it('recurses nested objects and marks single-leaf differences', () => {
    const t = makeT({ 'editor:fields.fullName': '姓名' });
    const local = { Fields: { fullName: { Name: '姓名', Type: 'text' } }, 全名: '123' };
    const remote = { Fields: { fullName: { Name: '姓名', Type: 'text' } }, 全名: '123ww' };
    const entries = buildDiffEntries('properties', local, remote, t);
    expect(entries?.filter((e) => e.changed)).toHaveLength(1);
    const changed = entries?.find((e) => e.changed);
    expect(changed?.label).toContain('全名');
    expect(changed?.localText).toBe('123');
    expect(changed?.remoteText).toBe('123ww');
  });

  it('marks leaves missing on one side as changed', () => {
    const t = makeT();
    const entries = buildDiffEntries('properties', { a: 1, b: 2 }, { a: 1 }, t);
    expect(entries).toHaveLength(2);
    const b = entries?.find((e) => e.label === 'B');
    expect(b?.localText).toBe('2');
    expect(b?.remoteText).toBeNull();
    expect(b?.changed).toBe(true);
    const a = entries?.find((e) => e.label === 'A');
    expect(a?.changed).toBe(false);
  });

  it('localizes booleans in leaf values', () => {
    const t = makeT({
      'settings:sync_conflict_value_true': '是',
      'settings:sync_conflict_value_false': '否',
    });
    const entries = buildDiffEntries('properties', { ok: true }, { ok: false }, t);
    expect(entries?.[0].localText).toBe('是');
    expect(entries?.[0].remoteText).toBe('否');
    expect(entries?.[0].changed).toBe(true);
  });

  it('exposes sensitivity tokens as badge levels', () => {
    const t = makeT();
    const entries = buildDiffEntries(
      'properties',
      { 出生日期: 'internal', 全名: 'public' },
      { 出生日期: 'internal', 全名: 'critical' },
      t,
    );
    const birth = entries?.find((e) => e.label === '出生日期');
    expect(birth?.localLevel).toBe('internal');
    expect(birth?.remoteLevel).toBe('internal');
    expect(birth?.changed).toBe(false);
    const name = entries?.find((e) => e.label === '全名');
    expect(name?.localLevel).toBe('public');
    expect(name?.remoteLevel).toBe('critical');
    expect(name?.changed).toBe(true);
  });

  it('keeps level null for non-sensitivity leaf values', () => {
    const t = makeT();
    const entries = buildDiffEntries('properties', { 出生日期: '2026-07-08' }, {}, t);
    expect(entries?.[0].localLevel).toBeNull();
    expect(entries?.[0].localText).toBe('2026-07-08');
  });

  it('i18n known leaf values and exposes icon components for icon fields', () => {
    const t = makeT({
      'settings:sync_conflict_tpltype_user': '用户模板',
      'settings:sync_conflict_icon_document': '文档',
    });
    const entries = buildDiffEntries(
      'properties',
      { template_type: 'user', icon_name: 'document' },
      {},
      t,
    );
    const tpl = entries?.find((e) => e.label === 'Template Type');
    expect(tpl?.localText).toBe('用户模板');
    expect(tpl?.localIcon).toBeNull();
    const icon = entries?.find((e) => e.label === 'Icon Name');
    expect(icon?.localText).toBe('文档');
    expect(icon?.localIcon).not.toBeNull();
  });

  it('expands arrays into indexed leaves', () => {
    const t = makeT();
    const entries = buildDiffEntries('childrenIds', ['a', 'b'], ['a', 'c'], t);
    expect(entries).toHaveLength(2);
    const second = entries?.find((e) => e.label === '[1]');
    expect(second?.localText).toBe('b');
    expect(second?.remoteText).toBe('c');
    expect(second?.changed).toBe(true);
  });
});
