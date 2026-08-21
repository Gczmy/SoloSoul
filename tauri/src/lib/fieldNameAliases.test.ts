import { describe, it, expect } from 'vitest';
import { resolveCanonicalFieldName } from './fieldNameAliases';

describe('resolveCanonicalFieldName（跨语言字段名归一）', () => {
  it('中文名与英文名映射到同一规范 id（出生日期 / Date of Birth）', () => {
    // 系统模板：key 相同、名字按语言本地化
    expect(resolveCanonicalFieldName('dateOfBirth', '出生日期')).toBe('dateOfBirth');
    expect(resolveCanonicalFieldName('dateOfBirth', 'Date of Birth')).toBe('dateOfBirth');
  });

  it('同名字不同 key 的跨语言匹配（ID Number → idNumber）', () => {
    // 自定义模板：不同 key（id_number / citizen_no），名字命中已知本地化名
    expect(resolveCanonicalFieldName('id_number', '身份证号')).toBe('idNumber');
    expect(resolveCanonicalFieldName('citizen_no', 'ID Number')).toBe('idNumber');
    expect(resolveCanonicalFieldName('citizen_no', '身份证号')).toBe('idNumber');
  });

  it('字段 key 命中已知规范 id 时优先用 key（模板重命名场景）', () => {
    // 系统字段被重命名为自定义名：名字不在别名表，但 key 仍是规范 id
    expect(resolveCanonicalFieldName('dateOfBirth', '我的生日')).toBe('dateOfBirth');
    expect(resolveCanonicalFieldName('passportNumber', 'Passport No.')).toBe('passportNumber');
  });

  it('未知自定义名字原样返回（去首尾空白）', () => {
    expect(resolveCanonicalFieldName('custom_key', '自定义字段')).toBe('自定义字段');
    expect(resolveCanonicalFieldName('custom_key', '  自定义字段  ')).toBe('自定义字段');
  });

  it('空名字回退为空字符串，空 key 不影响名字解析', () => {
    expect(resolveCanonicalFieldName('', '')).toBe('');
    expect(resolveCanonicalFieldName('', '出生日期')).toBe('dateOfBirth');
    expect(resolveCanonicalFieldName('', '自定义名')).toBe('自定义名');
  });

  it('更常用的系统字段别名覆盖', () => {
    expect(resolveCanonicalFieldName('full_name', '全名')).toBe('fullName');
    expect(resolveCanonicalFieldName('full_name', 'Full Name')).toBe('fullName');
    expect(resolveCanonicalFieldName('passport_no', '护照号码')).toBe('passportNumber');
    expect(resolveCanonicalFieldName('passport_no', 'Passport Number')).toBe('passportNumber');
    expect(resolveCanonicalFieldName('card_no', '卡号')).toBe('cardNumber');
    expect(resolveCanonicalFieldName('card_no', 'Card Number')).toBe('cardNumber');
  });
});
