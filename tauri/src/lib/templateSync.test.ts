import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { objectNeedsSync, resolveSemanticNeedsSync, __resetSemanticSyncCache } from './templateSync';
import { useObjectStore } from '@/stores/objectStore';

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
          {
            id: 'obj-1',
            templateId: 'tpl-1',
            templateHash: 'old-hash',
            ignoredTemplateHash: 'latest-hash',
          },
          map,
        ),
      ).toBe(false);
    });

    it('returns false when the object hash matches the latest hash', () => {
      const map = new Map([['tpl-1', 'latest-hash']]);
      expect(
        objectNeedsSync({ id: 'obj-1', templateId: 'tpl-1', templateHash: 'latest-hash' }, map),
      ).toBe(false);
    });
  });

  describe('resolveSemanticNeedsSync', () => {
    let previewSpy: ReturnType<typeof vi.fn>;
    let applySpy: ReturnType<typeof vi.fn>;

    beforeEach(() => {
      previewSpy = vi.fn();
      applySpy = vi.fn();
      __resetSemanticSyncCache();
      vi.spyOn(useObjectStore, 'getState').mockReturnValue({
        previewSyncTemplate: previewSpy,
        applySyncTemplate: applySpy,
      } as unknown as ReturnType<typeof useObjectStore.getState>);
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it('returns true when preview has real changes', async () => {
      previewSpy.mockResolvedValue({
        hasChanges: true,
        templateHash: 'latest-hash',
        fieldsAdded: [],
        fieldsDeprecated: [],
        fieldsUpdated: [],
        fieldsIncompatible: [],
      });
      const result = await resolveSemanticNeedsSync('acc-1', 'obj-1');
      expect(result).toBe(true);
      expect(previewSpy).toHaveBeenCalledWith('acc-1', 'obj-1');
      expect(applySpy).not.toHaveBeenCalled();
    });

    it('returns false and applies sync when preview has no changes', async () => {
      previewSpy.mockResolvedValue({
        hasChanges: false,
        templateHash: 'latest-hash',
        fieldsAdded: [],
        fieldsDeprecated: [],
        fieldsUpdated: [],
        fieldsIncompatible: [],
      });
      applySpy.mockResolvedValue(undefined);
      const result = await resolveSemanticNeedsSync('acc-1', 'obj-1');
      expect(result).toBe(false);
      expect(previewSpy).toHaveBeenCalledWith('acc-1', 'obj-1');
      expect(applySpy).toHaveBeenCalledWith('acc-1', 'obj-1');
    });

    it('returns true when preview rejects', async () => {
      previewSpy.mockRejectedValue(new Error('network error'));
      const result = await resolveSemanticNeedsSync('acc-1', 'obj-1');
      expect(result).toBe(true);
      expect(applySpy).not.toHaveBeenCalled();
    });
  });
});
