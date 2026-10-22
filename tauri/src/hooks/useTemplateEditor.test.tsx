import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useTemplateEditor } from './useTemplateEditor';

describe('template field creation', () => {
  it('creates the selected mobile type directly without waiting for a state update', () => {
    const { result } = renderHook(() => useTemplateEditor());
    let id = '';
    act(() => {
      result.current.openCreate();
      id = result.current.addPropertyOfType('email');
    });
    expect(result.current.editProperties).toEqual([
      expect.objectContaining({ id, type: 'email', sensitivityLevel: 'internal' }),
    ]);
    expect(result.current.newFieldType).toBe('text');
  });

  it('keeps the desktop creation path using its current selected type', () => {
    const { result } = renderHook(() => useTemplateEditor());
    act(() => result.current.openCreate());
    act(() => result.current.setNewFieldType('date'));
    act(() => result.current.addProperty());
    expect(result.current.editProperties[0].type).toBe('date');
  });
});
