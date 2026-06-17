import { describe, it, expect } from 'vitest';
import { CUSTOMIZABLE_ACTION_IDS, CUSTOMIZABLE_LINKS } from './useNavigationItems';

describe('useNavigationItems constants', () => {
  it('includes ocr in customizable action ids', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toContain('ocr');
  });

  it('does not map ocr to a link because it is a sidebar action', () => {
    expect(CUSTOMIZABLE_LINKS).not.toHaveProperty('ocr');
  });
});
