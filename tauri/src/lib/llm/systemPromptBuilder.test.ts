import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ObjectSummary } from '@/stores/objectStore';
import type { UserTemplate } from '@/types/template';
import { buildChatRequestMessages } from './chatRequest';

const state = vi.hoisted(() => ({
  objects: [] as ObjectSummary[],
  templates: [] as UserTemplate[],
}));
vi.mock('@/lib/i18n', () => ({ default: { language: 'zh-CN' } }));
vi.mock('@/stores/objectStore', () => ({ useObjectStore: { getState: () => state } }));
vi.mock('@/stores/templateStore', () => ({ useTemplateStore: { getState: () => state } }));
vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: { getState: () => ({ settings: {} }) },
}));
vi.mock('./guideService', () => ({
  searchGuideChunks: vi.fn().mockResolvedValue([]),
  formatChunksAsSystemMessage: vi.fn().mockReturnValue(null),
}));

function object(overrides: Partial<ObjectSummary> = {}): ObjectSummary {
  return {
    id: 'object',
    name: '公开对象',
    typeId: 'person',
    sensitivityLevel: 'public',
    createdAt: '',
    updatedAt: '',
    ...overrides,
  };
}

async function request(): Promise<string> {
  return JSON.stringify(
    await buildChatRequestMessages({
      text: '你好',
      history: [],
      includeSystemPrompt: true,
    }),
  );
}

describe('automatic public object context in final chat requests', () => {
  beforeEach(() => {
    state.objects = [];
    state.templates = [];
  });

  it('only includes explicitly public fields and never includes internal metadata', async () => {
    state.objects = [
      object({
        properties: {
          a: 'ALLOW',
          b: 'SECRET_INTERNAL',
          c: 'SECRET_SENSITIVE',
          d: 'SECRET_CRITICAL',
          e: 'SECRET_MISSING',
          f: 'SECRET_INVALID',
          __meta: 'SECRET_META',
        },
        propertyLabels: {
          a: 'public',
          b: 'internal',
          c: 'sensitive',
          d: 'critical',
          f: 'invalid',
          __meta: 'public',
        },
      }),
    ];
    const before = structuredClone(state.objects);
    const result = await request();
    expect(result).toContain('ALLOW');
    expect(result).not.toContain('SECRET_');
    expect(state.objects).toEqual(before);
  });

  it('uses labels before stored definitions before templates, failing closed for invalid labels', async () => {
    state.templates = [
      {
        id: 'template',
        properties: [
          { id: 'a', sensitivityLevel: 'public' },
          { id: 'b', sensitivityLevel: 'public' },
          { id: 'c', sensitivityLevel: 'public' },
          { id: 'd', sensitivityLevel: 'internal' },
        ],
      } as UserTemplate,
    ];
    state.objects = [
      object({
        templateId: 'template',
        properties: {
          a: 'SECRET_LABEL',
          b: 'SECRET_DEFINITION',
          c: 'ALLOW_TEMPLATE',
          d: 'ALLOW_LABEL',
          __fields: {
            a: { sensitivityLevel: 'public' },
            b: { sensitivityLevel: 'invalid' },
            d: { sensitivityLevel: 'critical' },
          },
        },
        propertyLabels: { a: '', d: 'public' },
      }),
    ];
    const result = await request();
    expect(result).not.toContain('SECRET_');
    expect(result).toContain('ALLOW_TEMPLATE');
    expect(result).toContain('ALLOW_LABEL');
  });

  it('uses persisted definitions after a template is removed and omits unknown fields', async () => {
    state.objects = [
      object({
        templateId: 'deleted',
        properties: {
          a: 'ALLOW_SNAPSHOT',
          b: 'SECRET_UNKNOWN',
          __fields: { a: { sensitivityLevel: 'public' } },
        },
      }),
    ];
    const result = await request();
    expect(result).toContain('ALLOW_SNAPSHOT');
    expect(result).not.toContain('SECRET_');
  });

  it('recursively filters dynamic groups without lowering parent protection', async () => {
    const children = [
      { name: 'ok', value: 'ALLOW_CHILD', sensitivityLevel: 'public' },
      { name: 'private', value: 'SECRET_CHILD', sensitivityLevel: 'critical' },
      { name: 'unknown', value: 'SECRET_MISSING' },
      { name: 'invalid', value: 'SECRET_INVALID', sensitivityLevel: 'invalid' },
      { name: '__metadata', value: 'SECRET_METADATA', sensitivityLevel: 'public' },
      {
        name: 'nested',
        type: 'dynamic_group',
        sensitivityLevel: 'public',
        value: [
          { name: 'ok', value: 'ALLOW_NESTED', sensitivityLevel: 'public' },
          { name: 'hidden', value: 'SECRET_NESTED', sensitivityLevel: 'internal' },
        ],
      },
      {
        name: 'locked',
        type: 'dynamic_group',
        sensitivityLevel: 'sensitive',
        value: [{ name: 'ok', value: 'SECRET_PARENT', sensitivityLevel: 'public' }],
      },
    ];
    state.objects = [
      object({
        properties: {
          group: JSON.stringify(children),
          hidden: children,
          __fields: {
            group: { type: 'dynamic_group', sensitivityLevel: 'public' },
            hidden: { type: 'dynamic_group', sensitivityLevel: 'critical' },
          },
        },
      }),
    ];
    const before = structuredClone(state.objects);
    const result = await request();
    expect(result).toContain('ALLOW_CHILD');
    expect(result).toContain('ALLOW_NESTED');
    expect(result).not.toContain('SECRET_');
    expect(state.objects).toEqual(before);
  });

  it('does not stringify untyped structures or malformed dynamic groups', async () => {
    state.objects = [
      object({
        properties: {
          a: { nested: 'SECRET_OBJECT' },
          b: [{ value: 'SECRET_ARRAY' }],
          c: 'SECRET_MALFORMED',
          d: ['ALLOW_ARRAY', 0, false],
          __fields: { c: { type: 'dynamic_group' } },
        },
        propertyLabels: { a: 'public', b: 'public', c: 'public', d: 'public' },
      }),
    ];
    const result = await request();
    expect(result).not.toContain('SECRET_');
    expect(result).toContain('ALLOW_ARRAY, 0, false');
  });

  it('filters before applying field limits, while retaining object-level restrictions', async () => {
    state.objects = [
      object({
        properties: {
          ...Object.fromEntries(Array.from({ length: 9 }, (_, i) => [`hidden${i}`, `SECRET_${i}`])),
          visible: 'ALLOW_AFTER_FILTER',
        },
        propertyLabels: { visible: 'public' },
      }),
      object({ name: 'SECRET_DELETED', isDeleted: true }),
      object({ name: 'SECRET_OBJECT', sensitivityLevel: 'internal' }),
    ];
    const result = await request();
    expect(result).toContain('ALLOW_AFTER_FILTER');
    expect(result).not.toContain('SECRET_');
  });
});
