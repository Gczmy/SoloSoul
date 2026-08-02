import { describe, it, expect } from 'vitest';
import { CUSTOMIZABLE_ACTION_IDS, CUSTOMIZABLE_LINKS } from './useNavigationItems';

describe('useNavigationItems constants', () => {
  it('includes ocr in customizable action ids', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toContain('ocr');
  });

  it('does not map ocr to a link because it is a sidebar action', () => {
    expect(CUSTOMIZABLE_LINKS).not.toHaveProperty('ocr');
  });

  it('CUSTOMIZABLE_ACTION_IDS has exactly 10 items', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toHaveLength(10);
    expect(CUSTOMIZABLE_ACTION_IDS).toEqual([
      'search',
      'trash',
      'templates',
      'attachments',
      'plugins',
      'ocr',
      'import_export',
      'sync',
      'help',
      'ai_chat',
    ]);
  });
});
