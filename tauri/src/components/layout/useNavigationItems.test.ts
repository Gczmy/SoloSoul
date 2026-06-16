import { describe, it, expect } from 'vitest';
import { CUSTOMIZABLE_ACTION_IDS, CUSTOMIZABLE_LINKS } from './useNavigationItems';

describe('useNavigationItems constants', () => {
  it('includes ocr in customizable action ids', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toContain('ocr');
  });

  it('maps ocr to /ocr route', () => {
    expect(CUSTOMIZABLE_LINKS.ocr).toEqual({
      path: '/ocr',
      iconKey: 'ocr',
      labelKey: 'ocr',
    });
  });
});
