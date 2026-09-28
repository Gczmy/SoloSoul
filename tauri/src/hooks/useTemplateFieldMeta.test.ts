import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useTemplateFieldMeta } from './useTemplateFieldMeta';

describe('RF302 历史字段敏感度使用真实标签值', () => {
  it('显式非法标签保守回退；仅缺失标签时读取模板；没有模板仍掩码', () => {
    const { result } = renderHook(() =>
      useTemplateFieldMeta([
        {
          id: 'template',
          properties: [{ id: 'field', name: 'Field', type: 'text', sensitivityLevel: 'public' }],
        },
      ]),
    );
    for (const label of [null, undefined, '', 'unknown', ['public'], { level: 'public' }]) {
      expect(result.current.getFieldSensitivity('template', 'field', { field: label })).toBe(
        'internal',
      );
    }
    expect(result.current.getFieldSensitivity('template', 'field', {})).toBe('public');
    expect(result.current.getFieldSensitivity('missing', 'field')).toBe('internal');
    expect(result.current.getFieldSensitivity('template', 'field', { field: 'critical' })).toBe(
      'critical',
    );
  });
});
