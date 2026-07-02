import { describe, it, expect, beforeAll } from 'vitest';
import { getTierLabel } from './utils';
import type { OcrTierInfo } from '@/lib/ipc';
import i18n from '@/lib/i18n';

const mockTier = (tier: string, name: string, description: string): OcrTierInfo => ({
  tier,
  name,
  description,
});

describe('getTierLabel', () => {
  beforeAll(async () => {
    await i18n.init({
      lng: 'en-US',
      fallbackLng: 'en-US',
      defaultNS: 'common',
      ns: ['common', 'ocr'],
      resources: {
        'en-US': {
          common: {},
          ocr: {
            tier_tiny_name: 'Tiny',
            tier_tiny_description: '1.5M parameters, fastest, suitable for simple scenes',
            tier_small_name: 'Small',
            tier_small_description: '~30MB, balanced speed and accuracy (default)',
            tier_medium_name: 'Medium',
            tier_medium_description: '~132MB, high accuracy, suitable for complex documents',
          },
        },
        'zh-CN': {
          common: {},
          ocr: {
            tier_tiny_name: 'Tiny',
            tier_tiny_description: '1.5M 参数，速度最快，适合简单场景',
            tier_small_name: 'Small',
            tier_small_description: '约 30MB，速度与精度平衡（默认）',
            tier_medium_name: 'Medium',
            tier_medium_description: '约 132MB，高精度，适合复杂文档',
          },
        },
      },
      interpolation: { escapeValue: false },
    });
  });

  it('returns localized name and description for known tiers', () => {
    const t = i18n.getFixedT('en-US', 'ocr');
    const label = getTierLabel(t, mockTier('small', 'Small', 'fallback desc'));
    expect(label.name).toBe('Small');
    expect(label.description).toBe('~30MB, balanced speed and accuracy (default)');
  });

  it('falls back to backend values for unknown tiers', () => {
    const t = i18n.getFixedT('en-US', 'ocr');
    const label = getTierLabel(t, mockTier('unknown', 'Backend Name', 'Backend Desc'));
    expect(label.name).toBe('Backend Name');
    expect(label.description).toBe('Backend Desc');
  });

  it('returns Chinese text for zh-CN locale', () => {
    const t = i18n.getFixedT('zh-CN', 'ocr');
    const label = getTierLabel(t, mockTier('medium', 'Medium', 'fallback'));
    expect(label.name).toBe('Medium');
    expect(label.description).toBe('约 132MB，高精度，适合复杂文档');
  });
});
