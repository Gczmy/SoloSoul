import { describe, expect, it } from 'vitest';
import { CUSTOM_ICON_MAP, resolveCustomIcon } from './pageIcons';

describe('resolveCustomIcon', () => {
  it('解析有效图标和历史别名', () => {
    expect(resolveCustomIcon('star')).toBe(CUSTOM_ICON_MAP.star);
    expect(resolveCustomIcon('passport')).toBe(CUSTOM_ICON_MAP.bookmarked);
  });

  it('原型链属性与未知 ID 均回退到默认图标', () => {
    expect(resolveCustomIcon('toString')).toBe(CUSTOM_ICON_MAP.document);
    expect(resolveCustomIcon('constructor')).toBe(CUSTOM_ICON_MAP.document);
    expect(resolveCustomIcon('missing')).toBe(CUSTOM_ICON_MAP.document);
  });
});
