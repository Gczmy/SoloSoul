import type { SensitivityLevel } from '@/types/template';

const levels: readonly SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

export function asFieldRecord(value: unknown): Record<string, unknown> | undefined {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

export interface FieldSensitivityInput {
  fieldId: string;
  propertyLabels?: Record<string, unknown>;
  definition?: { sensitivityLevel?: unknown };
  template?: { sensitivityLevel?: unknown };
  parent?: SensitivityLevel;
}

/** 保留原始来源，避免非法值归一为 internal 后被详情的明文例外误放行。 */
export function fieldSensitivityValue({
  fieldId,
  propertyLabels,
  definition,
  template,
}: FieldSensitivityInput): unknown {
  if (propertyLabels && Object.hasOwn(propertyLabels, fieldId)) return propertyLabels[fieldId];
  if (definition && Object.hasOwn(definition, 'sensitivityLevel'))
    return definition.sensitivityLevel;
  return template?.sensitivityLevel;
}

/** 显式非法值不能降级到后备来源的 public；父级保护只能加强。 */
export function resolveFieldSensitivity(input: FieldSensitivityInput): SensitivityLevel {
  const value = fieldSensitivityValue(input);
  const level = levels.includes(value as SensitivityLevel)
    ? (value as SensitivityLevel)
    : 'internal';
  const parent = input.parent;
  return parent && levels.indexOf(parent) > levels.indexOf(level) ? parent : level;
}
