import { asFieldRecord, resolveFieldSensitivity, fieldSensitivityValue } from './fieldSensitivity';
import { MASK_PLACEHOLDER } from './masking';
import type { SensitivityLevel } from '@/types/template';

export interface FieldPresentationPolicy {
  sensitivity: SensitivityLevel;
  concealed: boolean;
  requiresVerification: boolean;
}

/** 详情中的内部字段直接可读；摘要、搜索与回收站仍沿用默认遮罩。 */
export type FieldPresentationContext = 'summary' | 'detail';

export function fieldPresentationPolicy(
  input: Parameters<typeof resolveFieldSensitivity>[0],
  context: FieldPresentationContext = 'summary',
): FieldPresentationPolicy {
  const sensitivity = resolveFieldSensitivity(input);
  const source = fieldSensitivityValue(input);
  const visibleInternal =
    context === 'detail' &&
    sensitivity === 'internal' &&
    (source === 'internal' || (source === undefined && input.parent === 'internal'));
  return {
    sensitivity,
    concealed: sensitivity !== 'public' && !visibleInternal,
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
  context: FieldPresentationContext = 'summary',
): string {
  return sensitivity === 'public' ||
    (context === 'detail' && sensitivity === 'internal') ||
    revealed
    ? value
    : MASK_PLACEHOLDER;
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
