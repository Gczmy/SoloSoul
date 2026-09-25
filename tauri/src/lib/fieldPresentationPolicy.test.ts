import { describe, expect, it } from 'vitest';
import {
  dynamicFieldSensitivity,
  fieldPresentationPolicy,
  protectedDisplayValue,
} from './fieldPresentationPolicy';

describe('field presentation policy', () => {
  it.each(['public', 'internal', 'sensitive', 'critical', 'unknown', undefined])(
    'normalizes %s and conceals non-public values',
    (level) => {
      const policy = fieldPresentationPolicy({
        fieldId: 'f',
        definition: { sensitivityLevel: level },
      });
      expect(policy.concealed).toBe(level !== 'public');
      expect(policy.requiresVerification).toBe(level === 'critical');
      expect(protectedDisplayValue('secret', policy.sensitivity, false)).toBe(
        level === 'public' ? 'secret' : '••••••••',
      );
    },
  );
  it('keeps label/definition/template precedence and invalid labels fail closed', () => {
    expect(
      fieldPresentationPolicy({
        fieldId: 'f',
        propertyLabels: { f: 'bad' },
        definition: { sensitivityLevel: 'public' },
      }).sensitivity,
    ).toBe('internal');
    expect(
      fieldPresentationPolicy({
        fieldId: 'f',
        definition: { sensitivityLevel: 'critical' },
        template: { sensitivityLevel: 'public' },
      }).sensitivity,
    ).toBe('critical');
    expect(
      fieldPresentationPolicy({ fieldId: 'f', template: { sensitivityLevel: 'sensitive' } })
        .sensitivity,
    ).toBe('sensitive');
  });
  it('inherits parent strength and recursively protects dynamic child groups', () => {
    expect(dynamicFieldSensitivity({ sensitivityLevel: 'public' }, 'critical')).toBe('critical');
    expect(
      dynamicFieldSensitivity({
        type: 'dynamic_group',
        sensitivityLevel: 'public',
        value: JSON.stringify([{ type: 'text', sensitivityLevel: 'critical', value: 'secret' }]),
      }),
    ).toBe('critical');
    expect(
      dynamicFieldSensitivity({
        type: 'dynamic_group',
        sensitivityLevel: 'public',
        value: [{ value: 'unknown' }],
      }),
    ).toBe('internal');
  });
});
