import { describe, it, expect } from 'vitest';
import { objectNeedsSync } from './templateSync';

describe('templateSync', () => {
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

    it('returns false when the object hash matches the latest hash', () => {
      const map = new Map([['tpl-1', 'latest-hash']]);
      expect(
        objectNeedsSync(
          { id: 'obj-1', templateId: 'tpl-1', templateHash: 'latest-hash' },
          map,
        ),
      ).toBe(false);
    });
  });
});
