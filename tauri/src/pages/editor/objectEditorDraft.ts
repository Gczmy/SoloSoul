import type { ObjectData } from '@/stores/objectStore';

/** 与初次装配表单使用相同表示，保留动态字段组与多选数组。 */
export function objectFormValues(object: ObjectData): Record<string, unknown> {
  const values: Record<string, unknown> = {};
  const definitions = object.properties.__fields as Record<string, { type?: string }> | undefined;
  for (const [key, value] of Object.entries(object.properties)) {
    if (key.startsWith('__')) continue;
    if (definitions?.[key]?.type === 'dynamic_group' && Array.isArray(value)) values[key] = value;
    else if (typeof value === 'string' || Array.isArray(value)) values[key] = value;
    else if (value !== null && value !== undefined) values[key] = String(value);
  }
  return values;
}

function sameValue(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (Array.isArray(a) || Array.isArray(b))
    return (
      Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((value, i) => sameValue(value, b[i]))
    );
  if (!a || !b || typeof a !== 'object' || typeof b !== 'object') return false;
  const left = a as Record<string, unknown>;
  const right = b as Record<string, unknown>;
  const keys = Object.keys(left);
  return (
    keys.length === Object.keys(right).length &&
    keys.every((key) => Object.hasOwn(right, key) && sameValue(left[key], right[key]))
  );
}

export function objectEditorHasChanges(
  object: ObjectData,
  savedName: string,
  values: Record<string, unknown>,
): boolean {
  return savedName !== object.name || !sameValue(values, objectFormValues(object));
}
