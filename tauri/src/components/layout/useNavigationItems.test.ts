import { describe, it, expect } from 'vitest';
import { CUSTOMIZABLE_ACTION_IDS, CUSTOMIZABLE_LINKS, LOCK_ITEM, SETTINGS_ITEM } from './useNavigationItems';

describe('useNavigationItems constants', () => {
  it('includes ocr in customizable action ids', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toContain('ocr');
  });

  it('does not map ocr to a link because it is a sidebar action', () => {
    expect(CUSTOMIZABLE_LINKS).not.toHaveProperty('ocr');
  });

  it('exports LOCK_ITEM as an action with lock icon', () => {
    expect(LOCK_ITEM.type).toBe('action');
    expect(LOCK_ITEM.iconKey).toBe('lock');
    expect(LOCK_ITEM.labelKey).toBe('lock_vault');
  });

  it('exports SETTINGS_ITEM as a link with settings icon', () => {
    expect(SETTINGS_ITEM.type).toBe('link');
    expect(SETTINGS_ITEM.path).toBe('/settings');
    expect(SETTINGS_ITEM.iconKey).toBe('settings');
  });

  it('CUSTOMIZABLE_ACTION_IDS has exactly 8 items', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toHaveLength(8);
    expect(CUSTOMIZABLE_ACTION_IDS).toEqual([
      'search',
      'trash',
      'templates',
      'plugins',
      'ocr',
      'import_export',
      'help',
      'ai_chat',
    ]);
  });
});
