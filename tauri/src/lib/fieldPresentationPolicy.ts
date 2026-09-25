import { asFieldRecord, resolveFieldSensitivity } from './fieldSensitivity';
import { MASK_PLACEHOLDER } from './masking';
import type { SensitivityLevel } from '@/types/template';

export interface FieldPresentationPolicy {
  sensitivity: SensitivityLevel;
  concealed: boolean;
  requiresVerification: boolean;
}

export function fieldPresentationPolicy(
  input: Parameters<typeof resolveFieldSensitivity>[0],
): FieldPresentationPolicy {
  const sensitivity = resolveFieldSensitivity(input);
  return {
    sensitivity,
    concealed: sensitivity !== 'public',
    requiresVerification: sensitivity === 'critical',
  };
}

export function strongestSensitivity(levels: SensitivityLevel[]): SensitivityLevel {
  return levels.reduce(
    (parent, sensitivityLevel) =>
      resolveFieldSensitivity({ fieldId: '', definition: { sensitivityLevel }, parent }),
    'public',
  );
}

export function protectedDisplayValue(
  value: string,
  sensitivity: SensitivityLevel,
  revealed: boolean,
): string {
  return sensitivity === 'public' || revealed ? value : MASK_PLACEHOLDER;
}

/** 动态子组整组操作按后代最高级别授权，父级保护不可被子项降低。 */
export function dynamicFieldSensitivity(
  item: unknown,
  parent: SensitivityLevel = 'public',
  depth = 0,
): SensitivityLevel {
  const field = asFieldRecord(item);
  const level = resolveFieldSensitivity({
    fieldId: '',
    definition: { sensitivityLevel: field?.sensitivityLevel },
    parent,
  });
  if (!field || field.type !== 'dynamic_group') return level;
  if (depth >= 16) return 'critical';
  let children = field.value;
  if (typeof children === 'string') {
    try {
      children = JSON.parse(children);
    } catch {
      return strongestSensitivity([level, 'internal']);
    }
  }
  return Array.isArray(children)
    ? strongestSensitivity([
        level,
        ...children.map((child) => dynamicFieldSensitivity(child, level, depth + 1)),
      ])
    : strongestSensitivity([level, 'internal']);
}

/** 仅用于 React 内部实例隔离，不输出到 DOM/日志；完整值比较避免摘要碰撞。 */
export function fieldPresentationIdentity(
  accountId: string | undefined,
  objectId: string,
  fieldId: string,
  content: unknown,
): string {
  return JSON.stringify([accountId ?? null, objectId, fieldId, content]);
}
