import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TemplateEditor } from './TemplateEditor';
import type { TemplateProperty, PropertyType, SensitivityLevel } from '@/types/template';
import type { PluginManifest } from '@/lib/plugin';

// ── Mock deriveContractBindings ──────────────────────────────────────────

const mockDeriveContractBindings = vi.fn();
vi.mock('@/lib/plugin', async () => {
  const actual = await vi.importActual<typeof import('@/lib/plugin')>('@/lib/plugin');
  return {
    ...actual,
    deriveContractBindings: (...args: Parameters<typeof actual.deriveContractBindings>) =>
      mockDeriveContractBindings(...args),
  };
});

// ── Mock plugin store ────────────────────────────────────────────────────

const mockLoadInstalled = vi.fn().mockResolvedValue(undefined);
let mockInstalledPlugins: PluginManifest[] = [];

vi.mock('@/stores/pluginStore', () => ({
  usePluginStore: (selector: <T>(state: Record<string, unknown>) => T) => {
    const state = {
      installedPlugins: mockInstalledPlugins,
      loadInstalled: mockLoadInstalled,
    };
    return selector(state as unknown as Record<string, unknown>);
  },
}));

// ── Helpers ──────────────────────────────────────────────────────────────

function createMockPlugin(overrides: Partial<PluginManifest> = {}): PluginManifest {
  return {
    id: 'com.solosoul.test',
    name: 'Test Plugin',
    version: '1.0.0',
    description: 'A test plugin',
    author: 'SoloSoul',
    permissions: [],
    requiredCoreVersion: '>=2.5.0',
    wasmHashSha256: 'abc123',
    dataTtlSeconds: 3600,
    tier: 'p1',
    category: 'test',
    params: [],
    contracts: [],
    ...overrides,
  };
}

const FIELD_STREET: TemplateProperty = {
  id: 'street',
  name: 'Street',
  type: 'text' as PropertyType,
  sensitivityLevel: 'sensitive' as SensitivityLevel,
  contractField: true,
};

const FIELD_EMAIL: TemplateProperty = {
  id: 'email',
  name: 'Email',
  type: 'email' as PropertyType,
  sensitivityLevel: 'internal' as SensitivityLevel,
  // 非 contractField 字段
};

function createMockProps(overrides: Partial<Record<string, unknown>> = {}) {
  const handlers = {
    onEditNameChange: vi.fn(),
    onEditCategoryChange: vi.fn(),
    onEditIconIdChange: vi.fn(),
    onContractTypeIdChange: vi.fn(),
    onNewFieldTypeChange: vi.fn(),
    onAddProperty: vi.fn(),
    onUpdatePropertyName: vi.fn(),
    onUpdatePropertyType: vi.fn(),
    onUpdatePropertySensitivity: vi.fn(),
    onUpdatePropertyOptions: vi.fn(),
    onRemoveProperty: vi.fn(),
    onUpdatePropertyContractBindings: vi.fn(),
    onRestoreProperty: vi.fn(),
    onPermanentlyRemoveProperty: vi.fn(),
    onToggleShowDeprecated: vi.fn(),
    onSave: vi.fn(),
    onClose: vi.fn(),
  };

  return {
    editingTemplate: null,
    editName: '',
    editCategory: 'identity',
    editIconId: 'document',
    editContractTypeId: 'com.solosoul.test/v1',
    editProperties: [] as TemplateProperty[],
    newFieldType: 'text' as PropertyType,
    showDeprecated: false,
    fieldUsageMap: {} as Record<string, { active: number; softDeleted: number }>,
    ...handlers,
    ...overrides,
  } as Parameters<typeof TemplateEditor>[0] & typeof handlers;
}

// ── Tests ────────────────────────────────────────────────────────────────

describe('TemplateEditor — toggleBindingExpanded auto-derivation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInstalledPlugins = [];
  });

  it('auto-derives bindings on expand for contractField field without bindings', () => {
    mockInstalledPlugins = [
      createMockPlugin({
        id: 'com.solosoul.test',
        name: 'Test Plugin',
        contracts: [
          {
            typeId: 'com.solosoul.test/v1',
            version: 1,
            displayName: 'Test Contract',
            strictContractGate: false,
            typeIdAliases: [],
            roles: [
              { roleId: 'street', label: '街道', defaultPropertyId: 'street' },
              { roleId: 'city', label: '城市', defaultPropertyId: 'city' },
            ],
          },
        ],
      }),
    ];

    mockDeriveContractBindings.mockReturnValue([
      { contractTypeId: 'com.solosoul.test/v1', roleId: 'street' },
    ]);

    const props = createMockProps({ editProperties: [FIELD_STREET] });
    render(<TemplateEditor {...props} />);

    // 展开绑定面板
    fireEvent.click(screen.getByText('插件绑定'));

    // deriveContractBindings 被调用（参数正确）
    expect(mockDeriveContractBindings).toHaveBeenCalledWith(
      'com.solosoul.test/v1',
      'street',
      mockInstalledPlugins,
    );

    // 持久化回调被调用
    expect(props.onUpdatePropertyContractBindings).toHaveBeenCalledWith(0, [
      { contractTypeId: 'com.solosoul.test/v1', roleId: 'street' },
    ]);
  });

  it('does NOT auto-derive when field already has contractBindings', () => {
    mockInstalledPlugins = [
      createMockPlugin({
        contracts: [
          {
            typeId: 'com.solosoul.test/v1',
            version: 1,
            displayName: 'Test Contract',
            strictContractGate: false,
            typeIdAliases: [],
            roles: [{ roleId: 'street', label: '街道', defaultPropertyId: 'street' }],
          },
        ],
      }),
    ];

    const fieldWithBindings: TemplateProperty = {
      ...FIELD_STREET,
      contractBindings: [{ contractTypeId: 'com.solosoul.test/v1', roleId: 'street' }],
    };

    const props = createMockProps({ editProperties: [fieldWithBindings] });
    render(<TemplateEditor {...props} />);

    fireEvent.click(screen.getByText('插件绑定'));

    // 已有绑定 → 不触发持久化
    expect(props.onUpdatePropertyContractBindings).not.toHaveBeenCalled();
  });

  it('does NOT auto-derive for non-contractField fields', () => {
    mockInstalledPlugins = [
      createMockPlugin({
        contracts: [
          {
            typeId: 'com.solosoul.test/v1',
            version: 1,
            displayName: 'Test Contract',
            strictContractGate: false,
            typeIdAliases: [],
            roles: [{ roleId: 'email', label: 'Email', defaultPropertyId: 'email' }],
          },
        ],
      }),
    ];

    const props = createMockProps({ editProperties: [FIELD_EMAIL] });
    render(<TemplateEditor {...props} />);

    fireEvent.click(screen.getByText('插件绑定'));

    expect(props.onUpdatePropertyContractBindings).not.toHaveBeenCalled();
  });

  it('does NOT auto-derive when editContractTypeId is empty', () => {
    const props = createMockProps({
      editContractTypeId: '',
      editProperties: [FIELD_STREET],
    });
    render(<TemplateEditor {...props} />);

    fireEvent.click(screen.getByText('插件绑定'));

    expect(props.onUpdatePropertyContractBindings).not.toHaveBeenCalled();
  });

  it('does NOT persist when derived result is empty', () => {
    mockDeriveContractBindings.mockReturnValue([]);

    const props = createMockProps({ editProperties: [FIELD_STREET] });
    render(<TemplateEditor {...props} />);

    fireEvent.click(screen.getByText('插件绑定'));

    // derive 被调用，但空结果不应触发持久化
    expect(mockDeriveContractBindings).toHaveBeenCalled();
    expect(props.onUpdatePropertyContractBindings).not.toHaveBeenCalled();
  });

  it('collapse does NOT trigger persistence', () => {
    mockInstalledPlugins = [
      createMockPlugin({
        contracts: [
          {
            typeId: 'com.solosoul.test/v1',
            version: 1,
            displayName: 'Test Contract',
            strictContractGate: false,
            typeIdAliases: [],
            roles: [{ roleId: 'street', defaultPropertyId: 'street' }],
          },
        ],
      }),
    ];
    mockDeriveContractBindings.mockReturnValue([
      { contractTypeId: 'com.solosoul.test/v1', roleId: 'street' },
    ]);

    const props = createMockProps({ editProperties: [FIELD_STREET] });
    render(<TemplateEditor {...props} />);

    // 展开（触发一次持久化）
    fireEvent.click(screen.getByText('插件绑定'));
    expect(props.onUpdatePropertyContractBindings).toHaveBeenCalledTimes(1);

    // 折叠（不应再触发持久化）
    fireEvent.click(screen.getByText('插件绑定'));
    expect(props.onUpdatePropertyContractBindings).toHaveBeenCalledTimes(1);
  });

  it('shows derived bindings with dashed border and no remove button', () => {
    mockInstalledPlugins = [
      createMockPlugin({
        name: 'Address Plugin',
        contracts: [
          {
            typeId: 'com.solosoul.test/v1',
            version: 1,
            displayName: 'Address',
            strictContractGate: false,
            typeIdAliases: [],
            roles: [{ roleId: 'street', label: 'Street', defaultPropertyId: 'street' }],
          },
        ],
      }),
    ];
    mockDeriveContractBindings.mockReturnValue([
      { contractTypeId: 'com.solosoul.test/v1', roleId: 'street' },
    ]);

    const props = createMockProps({ editProperties: [FIELD_STREET] });
    const { container } = render(<TemplateEditor {...props} />);

    // 展开
    fireEvent.click(screen.getByText('插件绑定'));

    // 在 style 属性中找 dashed 标签
    const allElements = container.querySelectorAll<HTMLElement>('[style]');
    const derivedTag = Array.from(allElements).find(
      (el) => el.textContent?.includes('Street') && el.getAttribute('style')?.includes('dashed'),
    );
    expect(derivedTag).toBeTruthy();

    // 派生标签内不应有 button
    expect(derivedTag!.querySelector('button')).toBeFalsy();
  });

  it('shows persisted bindings with solid border and remove button', () => {
    mockInstalledPlugins = [
      createMockPlugin({
        name: 'Address Plugin',
        contracts: [
          {
            typeId: 'com.solosoul.test/v1',
            version: 1,
            displayName: 'Address',
            strictContractGate: false,
            typeIdAliases: [],
            roles: [{ roleId: 'street', label: 'Street', defaultPropertyId: 'street' }],
          },
        ],
      }),
    ];

    const fieldWithBinding: TemplateProperty = {
      ...FIELD_STREET,
      contractBindings: [{ contractTypeId: 'com.solosoul.test/v1', roleId: 'street' }],
    };

    const props = createMockProps({ editProperties: [fieldWithBinding] });
    const { container } = render(<TemplateEditor {...props} />);

    // 展开
    fireEvent.click(screen.getByText('插件绑定'));

    const allElements = container.querySelectorAll<HTMLElement>('[style]');
    const persistedTag = Array.from(allElements).find(
      (el) => el.textContent?.includes('Street') && el.getAttribute('style')?.includes('solid'),
    );
    expect(persistedTag).toBeTruthy();
    expect(persistedTag!.querySelector('button')).toBeTruthy();
  });
});
