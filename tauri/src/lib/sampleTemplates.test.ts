import { describe, it, expect } from 'vitest';
import { deriveSampleTemplateBindings, SAMPLE_TEMPLATES_ZH } from './sampleTemplates';
import type { PluginManifest } from './plugin';

const mockAddressFmtPlugin: PluginManifest = {
  id: 'com.solosoul.official.address-fmt',
  name: 'Address Formatter',
  version: '1.0.0',
  description: '格式化地址字段',
  author: 'SoloSoul',
  permissions: [],
  requiredCoreVersion: '>=2.5.0',
  wasmHashSha256: 'abc',
  dataTtlSeconds: 3600,
  tier: 'p1',
  category: 'formatting',
  params: [],
  contracts: [
    {
      typeId: 'com.solosoul.official.address-fmt/v1',
      version: 1,
      displayName: '地址契约',
      strictContractGate: false,
      typeIdAliases: [],
      roles: [
        { roleId: 'street', label: '街道', defaultPropertyId: 'street' },
        { roleId: 'district', label: '区/县', defaultPropertyId: 'district' },
        { roleId: 'city', label: '城市', defaultPropertyId: 'city' },
        { roleId: 'state', label: '省份', defaultPropertyId: 'state' },
        { roleId: 'country', label: '国家', defaultPropertyId: 'country' },
        { roleId: 'postalCode', label: '邮编', defaultPropertyId: 'postalCode' },
      ],
    },
  ],
};

const mockUnrelatedPlugin: PluginManifest = {
  id: 'com.solosoul.other',
  name: 'Other Plugin',
  version: '1.0.0',
  description: '无关插件',
  author: 'SoloSoul',
  permissions: [],
  requiredCoreVersion: '>=2.5.0',
  wasmHashSha256: 'def',
  dataTtlSeconds: 3600,
  tier: 'p2',
  category: 'other',
  params: [],
  contracts: [
    {
      typeId: 'com.solosoul.other/v1',
      version: 1,
      displayName: '其他契约',
      strictContractGate: false,
      typeIdAliases: [],
      roles: [{ roleId: 'name', label: '名称', defaultPropertyId: 'fullName' }],
    },
  ],
};

describe('deriveSampleTemplateBindings', () => {
  const addressTemplate = SAMPLE_TEMPLATES_ZH.find((t) => t.key === 'zh_address')!;

  it('为 address 模板所有 6 个 contractField: true 字段推导绑定', () => {
    const result = deriveSampleTemplateBindings(addressTemplate, [mockAddressFmtPlugin]);

    const street = result.find((p) => p.id === 'street');
    expect(street?.contractBindings).toHaveLength(1);
    expect(street?.contractBindings![0]).toEqual({
      contractTypeId: 'com.solosoul.official.address-fmt/v1',
      roleId: 'street',
    });

    const city = result.find((p) => p.id === 'city');
    expect(city?.contractBindings).toHaveLength(1);
    expect(city?.contractBindings![0].roleId).toBe('city');

    const postalCode = result.find((p) => p.id === 'postalCode');
    expect(postalCode?.contractBindings).toHaveLength(1);
    expect(postalCode?.contractBindings![0].roleId).toBe('postalCode');
  });

  it('无关字段（无 defaultPropertyId 匹配）不获得绑定', () => {
    // 模拟一个不匹配的 address 模板，其中有一个 fieldId 不在 role.defaultPropertyId 中
    const result = deriveSampleTemplateBindings(
      { ...addressTemplate, contractTypeId: 'com.solosoul.other/v1' },
      [mockUnrelatedPlugin],
    );

    // 所有字段的 id 与 other 契约的 role.fullName 都不匹配，应无绑定
    for (const prop of result) {
      expect(prop.contractBindings).toBeUndefined();
    }
  });

  it('contractField: true 的字段若无匹配插件，继承原样无绑定', () => {
    const result = deriveSampleTemplateBindings(addressTemplate, []);

    for (const prop of result) {
      expect(prop.contractBindings).toBeUndefined();
    }
  });

  it('contractTypeId 不存在时不做推导', () => {
    const templateNoContract = { ...addressTemplate, contractTypeId: undefined };
    const result = deriveSampleTemplateBindings(templateNoContract, [mockAddressFmtPlugin]);

    for (const prop of result) {
      expect(prop.contractBindings).toBeUndefined();
    }
  });

  it('非 contractField 的字段不受影响', () => {
    const result = deriveSampleTemplateBindings(addressTemplate, [mockAddressFmtPlugin]);

    const district = result.find((p) => p.id === 'district');
    expect(district?.contractBindings).toHaveLength(1); // 是 contractField: true

    // 其他模板中的非 contractField 字段不应受影响
    const identityTemplate = SAMPLE_TEMPLATES_ZH.find((t) => t.key === 'zh_identity')!;
    const identityResult = deriveSampleTemplateBindings(identityTemplate, [mockAddressFmtPlugin]);

    const fullName = identityResult.find((p) => p.id === 'fullName');
    expect(fullName?.contractBindings).toBeUndefined();
  });

  it('已有 contractBindings 的字段不会被覆盖', () => {
    // 预填一个已有绑定的字段，模拟持久化后的回读
    const templateWithExisting = {
      ...addressTemplate,
      properties: addressTemplate.properties.map((p) =>
        p.id === 'street'
          ? {
              ...p,
              contractBindings: [{ contractTypeId: 'custom/v1', roleId: 'customRole' }],
            }
          : p,
      ),
    };
    const result = deriveSampleTemplateBindings(templateWithExisting, [mockAddressFmtPlugin]);

    const street = result.find((p) => p.id === 'street');
    expect(street?.contractBindings).toHaveLength(1);
    // 不应被推导覆盖
    expect(street?.contractBindings![0].contractTypeId).toBe('custom/v1');
    expect(street?.contractBindings![0].roleId).toBe('customRole');
  });
});
