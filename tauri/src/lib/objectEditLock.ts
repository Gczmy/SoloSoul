/** 保留元数据；缺失或显式 false 可编辑，异常标记保持写保护。 */
export function objectEditLocked(properties?: unknown): boolean {
  return (
    typeof properties === 'object' &&
    properties !== null &&
    Object.hasOwn(properties, '__editLocked') &&
    '__editLocked' in properties &&
    properties.__editLocked !== false
  );
}
