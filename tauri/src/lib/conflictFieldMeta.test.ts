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
  shouldOmitField,
} from './conflictFieldMeta';

/** 模拟 i18n：settings:/editor: 命中返回假想译文，未命中返回 defaultValue；支持 {{count}} 插值。 */
function makeT(overrides: Record<string, string> = {}) {
  return vi.fn((key: string, opts?: { defaultValue?: string; count?: number }) => {
    const tpl = overrides[key] ?? opts?.defaultValue ?? key;
    return typeof opts?.count === 'number'
      ? tpl.replace(/\{\{count\}\}/g, String(opts.count))
      : tpl;
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

  it('localizes property type values via editor:field_types', () => {
    const t = makeT({
      'editor:field_types.date': '日期',
      'editor:field_types.email': '邮箱',
    });
    expect(formatConflictValue('type', 'date', t)).toBe('日期');
    expect(formatConflictValue('type', 'email', t)).toBe('邮箱');
    // 未知类型保持原值
    expect(formatConflictValue('type', 'custom', t)).toBe('custom');
  });

  it('trims time fields to second precision', () => {
    const t = makeT();
    expect(formatConflictValue('createdAt', '2026-08-05T12:34:56.789Z', t)).toBe(
      '2026-08-05T12:34:56Z',
    );
    expect(formatConflictValue('updated_at', '2026-08-05T12:34:56.123456+00:00', t)).toBe(
      '2026-08-05T12:34:56Z',
    );
    expect(formatConflictValue('deletedAt', '2026-08-05T12:34:56Z', t)).toBe(
      '2026-08-05T12:34:56Z',
    );
    expect(formatConflictValue('expiresAt', '2026-08-05T12:34:56.000+08:00', t)).toBe(
      '2026-08-05T12:34:56+08:00',
    );
  });

  it('keeps non-RFC3339 strings and non-time fields unchanged', () => {
    const t = makeT();
    // 用户日期字段（非时间元数据）不受影响
    expect(formatConflictValue('birthDate', '2026-07-08T10:00:00.123Z', t)).toBe(
      '2026-07-08T10:00:00.123Z',
    );
    // 非 RFC3339 时间值原样
    expect(formatConflictValue('createdAt', '2026-08-05', t)).toBe('2026-08-05');
    expect(formatConflictValue('createdAt', 1234567890, t)).toBe('1234567890');
  });
});

describe('shouldOmitField', () => {
  it('always omits object fingerprint hashes', () => {
    expect(shouldOmitField('templateHash', 'abc', 'def')).toBe(true);
    expect(shouldOmitField('template_hash', null, null)).toBe(true);
    expect(shouldOmitField('ignored_template_hash', 'x', undefined)).toBe(true);
  });

  it('omits children_ids when both sides have no children', () => {
    expect(shouldOmitField('childrenIds', [], [])).toBe(true);
    expect(shouldOmitField('children_ids', undefined, [])).toBe(true);
  });

  it('keeps children_ids when a real relationship difference exists', () => {
    expect(shouldOmitField('childrenIds', ['obj-1'], [])).toBe(false);
    expect(shouldOmitField('childrenIds', ['obj-1'], ['obj-2'])).toBe(false);
  });

  it('omits parent_id when both sides have no parent', () => {
    expect(shouldOmitField('parentId', null, null)).toBe(true);
    expect(shouldOmitField('parent_id', undefined, '')).toBe(true);
  });

  it('keeps parent_id when a real parent difference exists', () => {
    expect(shouldOmitField('parentId', 'p-1', null)).toBe(false);
    expect(shouldOmitField('parentId', 'p-1', 'p-2')).toBe(false);
  });

  it('keeps meaningful user fields', () => {
    expect(shouldOmitField('name', '张三', '李四')).toBe(false);
    expect(shouldOmitField('templateId', 'identity', 'passport')).toBe(false);
    expect(shouldOmitField('properties', { a: 1 }, { a: 2 })).toBe(false);
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

  it('collapses identical __fields schema into a single summary entry', () => {
    const t = makeT({
      'settings:sync_conflict_field_schema': '字段定义',
      'settings:sync_conflict_field_schema_count': '共 {{count}} 项',
    });
    const fields = {
      fullName: { name: '姓名', type: 'text' },
      email: { name: '邮箱', type: 'email' },
    };
    const local = { __fields: fields, __templateName: '身份信息', fullName: '张三', email: 'a@b.com' };
    const remote = { __fields: fields, __templateName: '身份信息', fullName: '张三', email: 'a@c.com' };
    const entries = buildDiffEntries('properties', local, remote, t);
    expect(entries).not.toBeNull();
    // 摘要条目：字段定义 + 共 2 项，无差异
    const schema = entries?.find((e) => e.path === '__fields');
    expect(schema?.label).toBe('字段定义');
    expect(schema?.localText).toBe('共 2 项');
    expect(schema?.changed).toBe(false);
    // 不再展开 __fields 叶子
    expect(entries?.some((e) => e.path.includes('› name'))).toBe(false);
    expect(entries?.some((e) => e.path.includes('› type'))).toBe(false);
    // 真实字段值仍逐叶展开并标记差异
    const email = entries?.find((e) => e.path === 'email');
    expect(email?.localText).toBe('a@b.com');
    expect(email?.remoteText).toBe('a@c.com');
    expect(email?.changed).toBe(true);
  });

  it('expands __fields when the schema actually differs', () => {
    const t = makeT({ 'settings:sync_conflict_field_schema': '字段定义' });
    const local = { __fields: { fullName: { name: '姓名', type: 'text' } }, 全名: '123' };
    const remote = { __fields: { fullName: { name: '姓名', type: 'multiline' } }, 全名: '123' };
    const entries = buildDiffEntries('properties', local, remote, t);
    expect(entries).not.toBeNull();
    // 无摘要条目（有差异 → 展开）
    expect(entries?.some((e) => e.path === '__fields')).toBe(false);
    // 展开的叶子暴露 type 差异，前缀标签为「字段定义」
    const typeLeaf = entries?.find((e) => e.path.endsWith('› type'));
    expect(typeLeaf?.localText).toBe('text');
    expect(typeLeaf?.remoteText).toBe('multiline');
    expect(typeLeaf?.changed).toBe(true);
    expect(typeLeaf?.label).toContain('字段定义');
  });

  it('i18n type codes in expanded __fields leaves', () => {
    const t = makeT({
      'settings:sync_conflict_field_schema': '字段定义',
      'editor:field_types.text': '文本',
      'editor:field_types.multiline': '多行文本',
    });
    const local = { __fields: { fullName: { name: '姓名', type: 'text' } } };
    const remote = { __fields: { fullName: { name: '姓名', type: 'multiline' } } };
    const entries = buildDiffEntries('properties', local, remote, t);
    const typeLeaf = entries?.find((e) => e.path.endsWith('› type'));
    expect(typeLeaf?.localText).toBe('文本');
    expect(typeLeaf?.remoteText).toBe('多行文本');
    expect(typeLeaf?.changed).toBe(true);
  });

  it('expands __fields present on one side only', () => {
    const t = makeT({ 'settings:sync_conflict_field_schema': '字段定义' });
    const local = { __fields: { a: { name: 'A', type: 'text' } } };
    const remote = {};
    const entries = buildDiffEntries('properties', local, remote, t);
    expect(entries?.some((e) => e.path === '__fields' && e.changed)).toBe(false);
    // 单侧缺失的叶子全部标为差异
    expect(entries?.some((e) => e.path.includes('a') && e.changed)).toBe(true);
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

  it('trims time values to seconds in expanded leaves', () => {
    const t = makeT();
    const entries = buildDiffEntries(
      'properties',
      { created_at: '2026-08-05T12:34:56.789Z' },
      { created_at: '2026-08-05T12:34:56.789Z' },
      t,
    );
    const leaf = entries?.find((e) => e.path === 'created_at');
    expect(leaf?.localText).toBe('2026-08-05T12:34:56Z');
    expect(leaf?.remoteText).toBe('2026-08-05T12:34:56Z');
    expect(leaf?.changed).toBe(false);
  });
});
