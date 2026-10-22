import type { useTemplateEditor } from '@/hooks/useTemplateEditor';
import type { PropertyType, SensitivityLevel } from '@/types/template';

export type TemplateEditorState = ReturnType<typeof useTemplateEditor>;

export type TemplatePanel =
  | { kind: 'page' | 'icon' | 'add' | 'dynamic' }
  | { kind: 'type' | 'sensitivity' | 'options' | 'binding' | 'contract' | 'role'; fieldId: string };

export const FIELD_TYPES: PropertyType[] = [
  'text',
  'multiline',
  'number',
  'date',
  'datetime',
  'boolean',
  'select',
  'multiselect',
  'url',
  'email',
  'phone',
  'file',
];
export const SENSITIVITY_LEVELS: SensitivityLevel[] = [
  'public',
  'internal',
  'sensitive',
  'critical',
];

export const SYSTEM_PAGES = ['identity', 'travel', 'financial', 'professional'] as const;
