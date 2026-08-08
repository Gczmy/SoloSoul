import { useCallback, useMemo } from 'react';
import type { TemplateProperty } from '@/types/template';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';

/**
 * P013/5: 模板字段元数据 O(1) 查找（F011 缓存 + 敏感度/废弃/显示名解析）。
 */
export function useTemplateFieldMeta(userTemplates: Array<{ id: string; properties: TemplateProperty[] }>) {
  // F011: cache template field metadata so lookups are O(1) instead of O(n²).
  const templateFieldMap = useMemo(() => {
    const map = new Map<string, Map<string, TemplateProperty>>();
    for (const t of userTemplates) {
      map.set(t.id, new Map(t.properties.map((p) => [p.id, p])));
    }
    return map;
  }, [userTemplates]);

  const getFieldProperty = useCallback(
    (templateId: string | undefined, fieldKey: string): TemplateProperty | undefined => {
      return templateFieldMap.get(templateId || '')?.get(fieldKey);
    },
    [templateFieldMap],
  );

  const getFieldSensitivity = useCallback(
    (
      templateId: string | undefined,
      fieldKey: string,
      propertyLabels?: Record<string, string>,
    ): SensitivityLevel => {
      // 1. 对象自有 propertyLabels（即使模板被删除也保留敏感度）
      if (propertyLabels?.[fieldKey]) {
        return propertyLabels[fieldKey] as SensitivityLevel;
      }
      // 2. 回退到模板定义
      return (
        (getFieldProperty(templateId, fieldKey)?.sensitivityLevel as SensitivityLevel) || 'public'
      );
    },
    [getFieldProperty],
  );

  const isFieldDeprecated = useCallback(
    (templateId: string | undefined, fieldKey: string): boolean => {
      return !!getFieldProperty(templateId, fieldKey)?.deprecatedAt;
    },
    [getFieldProperty],
  );

  const getFieldName = useCallback(
    (
      templateId: string | undefined,
      fieldKey: string,
      propertyFields?: Record<string, { name: string }>,
    ): string => {
      return (
        getFieldProperty(templateId, fieldKey)?.name || propertyFields?.[fieldKey]?.name || fieldKey
      );
    },
    [getFieldProperty],
  );

  return {
    getFieldSensitivity,
    isFieldDeprecated,
    getFieldName,
  };
}
