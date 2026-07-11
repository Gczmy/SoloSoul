import { describe, it, expect } from 'vitest';
import {
  computeTemplateFingerprint,
  buildTemplateHashMap,
  objectNeedsSync,
} from './templateSync';
import type { UserTemplate } from '@/types/template';

function makeTemplate(overrides?: Partial<UserTemplate>): UserTemplate {
  return {
    id: 'tpl-1',
    accountId: 'acc-1',
    name: 'Contact',
    iconId: 'user',
    category: 'identity',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-06-01T00:00:00Z',
    properties: [
      {
        id: 'name',
        name: 'Name',
        type: 'text',
        sensitivityLevel: 'internal',
      },
      {
        id: 'email',
        name: 'Email',
        type: 'email',
        sensitivityLevel: 'internal',
      },
    ],
    ...overrides,
  };
}

describe('templateSync', () => {
  describe('computeTemplateFingerprint', () => {
    it('returns the same hash for identical templates', async () => {
      const a = makeTemplate();
      const b = makeTemplate();
      const hashA = await computeTemplateFingerprint(a);
      const hashB = await computeTemplateFingerprint(b);
      expect(hashA).toBe(hashB);
      expect(hashA).toHaveLength(16);
    });

    it('returns a different hash when a property changes', async () => {
      const original = makeTemplate();
      const changed = makeTemplate({
        properties: [
          { id: 'name', name: 'Full Name', type: 'text', sensitivityLevel: 'internal' },
          { id: 'email', name: 'Email', type: 'email', sensitivityLevel: 'internal' },
        ],
      });
      const hashOriginal = await computeTemplateFingerprint(original);
      const hashChanged = await computeTemplateFingerprint(changed);
      expect(hashOriginal).not.toBe(hashChanged);
    });

    it('returns a different hash when a property sensitivity changes', async () => {
      const original = makeTemplate();
      const changed = makeTemplate({
        properties: [
          { id: 'name', name: 'Name', type: 'text', sensitivityLevel: 'sensitive' },
          { id: 'email', name: 'Email', type: 'email', sensitivityLevel: 'internal' },
        ],
      });
      const hashOriginal = await computeTemplateFingerprint(original);
      const hashChanged = await computeTemplateFingerprint(changed);
      expect(hashOriginal).not.toBe(hashChanged);
    });

    it('matches backend serialization keys (camelCase sensitivityLevel / deprecatedAt)', async () => {
      const tpl = makeTemplate({
        properties: [
          {
            id: 'a',
            name: 'A',
            type: 'text',
            sensitivityLevel: 'internal',
            deprecatedAt: '2024-01-01T00:00:00Z',
          },
        ],
      });
      const hash = await computeTemplateFingerprint(tpl);
      // 后端 TemplateProperty 使用 #[serde(rename_all = "camelCase")] 序列化，
      // 因此敏感度与废弃时间字段应为 sensitivityLevel / deprecatedAt。
      expect(hash).toBe('ca53fd3a572675e8');
    });

    it('matches backend property field order (options before deprecatedAt)', async () => {
      const tpl = makeTemplate({
        properties: [
          {
            id: 'a',
            name: 'A',
            type: 'select',
            sensitivityLevel: 'internal',
            options: ['x', 'y'],
            deprecatedAt: '2024-01-01T00:00:00Z',
          },
        ],
      });
      const hash = await computeTemplateFingerprint(tpl);
      // 后端 struct 顺序：id → name → type → sensitivity_level → options → deprecated_at → ...
      // 对应 JSON：{"properties":[{"id":"a","name":"A","type":"select","sensitivityLevel":"internal","options":["x","y"],"deprecatedAt":"2024-01-01T00:00:00Z"}]}
      expect(hash).toBe('0fb63198abec968f');
    });

    it('returns a different hash when contractBindings change', async () => {
      const original = makeTemplate({
        properties: [
          { id: 'name', name: 'Name', type: 'text', sensitivityLevel: 'internal' },
        ],
      });
      const changed = makeTemplate({
        properties: [
          {
            id: 'name',
            name: 'Name',
            type: 'text',
            sensitivityLevel: 'internal',
            contractBindings: [{ contractTypeId: 'plugin', roleId: 'role' }],
          },
        ],
      });
      const hashOriginal = await computeTemplateFingerprint(original);
      const hashChanged = await computeTemplateFingerprint(changed);
      expect(hashOriginal).not.toBe(hashChanged);
    });

    it('returns the same hash regardless of property order', async () => {
      const a = makeTemplate();
      const b = makeTemplate({
        properties: [...a.properties].reverse(),
      });
      const hashA = await computeTemplateFingerprint(a);
      const hashB = await computeTemplateFingerprint(b);
      expect(hashA).toBe(hashB);
    });

    it('ignores template id and timestamps', async () => {
      const a = makeTemplate();
      const b = makeTemplate({
        id: 'tpl-2',
        accountId: 'acc-2',
        createdAt: '2020-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      });
      const hashA = await computeTemplateFingerprint(a);
      const hashB = await computeTemplateFingerprint(b);
      expect(hashA).toBe(hashB);
    });
  });

  describe('buildTemplateHashMap', () => {
    it('builds a map of template id to hash', async () => {
      const t1 = makeTemplate({ id: 'tpl-1' });
      const t2 = makeTemplate({ id: 'tpl-2', properties: [{ id: 'f2', name: 'Field 2', type: 'text' }] });
      const map = await buildTemplateHashMap([t1, t2]);
      expect(map.size).toBe(2);
      expect(map.get('tpl-1')).toHaveLength(16);
      expect(map.get('tpl-2')).toHaveLength(16);
      expect(map.get('tpl-1')).not.toBe(map.get('tpl-2'));
    });
  });

  describe('objectNeedsSync', () => {
    it('returns false when the object has no template id', () => {
      const map = new Map([['tpl-1', 'abc']]);
      expect(objectNeedsSync({ id: 'obj-1' }, map)).toBe(false);
    });

    it('returns false when the template is not in the map', () => {
      const map = new Map<string, string>();
      expect(objectNeedsSync({ id: 'obj-1', templateId: 'tpl-missing' }, map)).toBe(false);
    });

    it('returns true when the object has no template hash', () => {
      const map = new Map([['tpl-1', 'abc']]);
      expect(objectNeedsSync({ id: 'obj-1', templateId: 'tpl-1' }, map)).toBe(true);
    });

    it('returns true when the hash differs', () => {
      const map = new Map([['tpl-1', 'latest-hash']]);
      expect(
        objectNeedsSync({ id: 'obj-1', templateId: 'tpl-1', templateHash: 'old-hash' }, map),
      ).toBe(true);
    });

    it('returns false when the ignored hash matches the latest hash', () => {
      const map = new Map([['tpl-1', 'latest-hash']]);
      expect(
        objectNeedsSync(
          { id: 'obj-1', templateId: 'tpl-1', templateHash: 'old-hash', ignoredTemplateHash: 'latest-hash' },
          map,
        ),
      ).toBe(false);
    });
  });
});
