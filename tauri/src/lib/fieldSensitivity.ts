import type { SensitivityLevel } from '@/types/template';

const levels: readonly SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

export function asFieldRecord(value: unknown): Record<string, unknown> | undefined {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

/** 显式非法值不能降级到后备来源的 public；父级保护只能加强。 */
export function resolveFieldSensitivity({
  fieldId,
  propertyLabels,
  definition,
  template,
  parent,
}: {
  fieldId: string;
  propertyLabels?: Record<string, unknown>;
  definition?: { sensitivityLevel?: unknown };
  template?: { sensitivityLevel?: unknown };
  parent?: SensitivityLevel;
}): SensitivityLevel {
  let value: unknown;
  if (propertyLabels && Object.hasOwn(propertyLabels, fieldId)) {
    value = propertyLabels[fieldId];
  } else if (definition && Object.hasOwn(definition, 'sensitivityLevel')) {
    value = definition.sensitivityLevel;
  } else {
    value = template?.sensitivityLevel;
  }
  const level = levels.includes(value as SensitivityLevel)
    ? (value as SensitivityLevel)
    : 'internal';
  return parent && levels.indexOf(parent) > levels.indexOf(level) ? parent : level;
}
