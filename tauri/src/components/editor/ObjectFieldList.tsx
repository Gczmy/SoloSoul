import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { PluginBadge } from '@/components/template/PluginBadge';
import { TemplateFieldInput } from '@/components/TemplateFieldInput';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import type { PropertyType } from '@/types/template';
import type { ObjectData } from '@/stores/objectStore';

interface FieldDef {
  key: string;
  label: string;
  type: string;
  options?: string[];
  sensitivityLevel?: string;
  required?: boolean;
  deprecatedAt?: string;
  contractField?: boolean;
}

interface ObjectFieldListProps {
  fields: FieldDef[];
  displayFields: FieldDef[];
  values: Record<string, unknown>;
  onChange: (key: string, val: unknown) => void;
  validationErrors: Record<string, string>;
  onClearError: (key: string) => void;
  currentObject?: ObjectData | null;
  contractTypeId?: string;
  getSensitivity: (fieldKey: string, templateDefault?: string) => SensitivityLevel;
  isNew: boolean;
}

export function ObjectFieldList({
  fields,
  displayFields,
  values,
  onChange,
  validationErrors,
  onClearError,
  currentObject,
  contractTypeId,
  getSensitivity,
  isNew: _isNew,
}: ObjectFieldListProps) {
  const { t } = useTranslation(['common', 'editor', 'navigation']);

  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 12 }}>
        {t('common:properties')}
      </h3>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {fields.length === 0
          ? // Fallback: template deleted, use __fields for type-aware rendering
            Object.entries(values)
              .filter(([k]) => !k.startsWith('__'))
              .map(([key, val]) => {
                const fieldDefs = (currentObject?.properties as Record<string, unknown>)
                  ?.__fields as
                  | Record<
                      string,
                      {
                        name: string;
                        type: string;
                        options?: string[];
                        deprecatedAt?: string;
                        contractField?: boolean;
                      }
                    >
                  | undefined;
                const fieldDef = fieldDefs?.[key];
                const fieldName = fieldDef?.name || key;
                const propType: PropertyType = (fieldDef?.type as PropertyType) || 'text';
                const isDeprecated = !!fieldDef?.deprecatedAt;
                const objLabels = currentObject?.propertyLabels as
                  | Record<string, string>
                  | undefined;
                const sensitivity: SensitivityLevel =
                  (objLabels?.[key] as SensitivityLevel) || 'internal';
                const isContractField = fieldDef?.contractField === true;
                const objContractTypeId = currentObject?.contractTypeId;
                return (
                  <div key={key}>
                    <TemplateFieldInput
                      propertyId={key}
                      label={fieldName}
                      type={propType}
                      options={fieldDef?.options}
                      value={val}
                      icon={<FieldTypeIcon type={propType} />}
                      badge={
                        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                          <SensitivityBadge level={sensitivity} />
                          {isContractField && objContractTypeId && (
                            <PluginBadge
                              contractTypeId={objContractTypeId}
                              size="sm"
                              variant="full"
                            />
                          )}
                          {isDeprecated && <DeprecatedBadge />}
                        </div>
                      }
                      hint={
                        ['email', 'url', 'phone', 'date', 'number'].includes(propType)
                          ? t(`editor:validation_hint_${propType}`)
                          : undefined
                      }
                      onChange={(val) => {
                        onChange(key, val);
                        if (validationErrors[key]) onClearError(key);
                      }}
                    />
                    {validationErrors[key] && (
                      <div
                        style={{ fontSize: 'var(--text-badge)', color: '#ef4444', marginTop: 4 }}
                      >
                        {validationErrors[key]}
                      </div>
                    )}
                  </div>
                );
              })
          : displayFields.map((field) => {
              const sensitivity = getSensitivity(field.key, field.sensitivityLevel);
              const fieldLabel = field.label;
              const propType: PropertyType =
                field.type === 'tel'
                  ? 'phone'
                  : field.type === 'datetime-local'
                    ? 'datetime'
                    : (field.type as PropertyType) || 'text';
              const isDeprecated = !!field.deprecatedAt;
              return (
                <div key={field.key}>
                  <TemplateFieldInput
                    propertyId={field.key}
                    label={fieldLabel}
                    type={propType}
                    options={field.options}
                    value={values[field.key]}
                    icon={<FieldTypeIcon type={propType} />}
                    badge={
                      <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                        <SensitivityBadge level={sensitivity} />
                        {field.contractField && contractTypeId && (
                          <PluginBadge contractTypeId={contractTypeId} size="sm" variant="full" />
                        )}
                        {isDeprecated && <DeprecatedBadge />}
                      </div>
                    }
                    hint={
                      ['email', 'url', 'phone', 'date', 'number'].includes(propType)
                        ? t(`editor:validation_hint_${propType}`)
                        : undefined
                    }
                    onChange={(val) => {
                      onChange(field.key, val);
                      if (validationErrors[field.key]) onClearError(field.key);
                    }}
                  />
                  {validationErrors[field.key] && (
                    <div style={{ fontSize: 'var(--text-badge)', color: '#ef4444', marginTop: 4 }}>
                      {validationErrors[field.key]}
                    </div>
                  )}
                </div>
              );
            })}
      </div>
    </Card>
  );
}
