import { Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type {
  ContractRoleBinding,
  PropertyType,
  SensitivityLevel,
  TemplateProperty,
} from '@/types/template';
import { TemplateTypeSelect } from './TemplateTypeSelect';
import { DynamicGroupConfig } from './DynamicGroupConfig';
import { TemplateFieldRow, type FlattenedContract } from './TemplateFieldRow';
import { DeprecatedFieldsSection, type FieldUsage } from './DeprecatedFieldsSection';
import type { PluginManifest } from '@/lib/plugin';
import { ICON_SIZE } from '@/lib/constants';

interface TemplateFieldsPanelProps {
  editProperties: TemplateProperty[];
  activeRows: { prop: TemplateProperty; idx: number }[];
  expandedBindingFields: Set<string>;
  editContractTypeId: string;
  installedPlugins: PluginManifest[];
  flattenContracts: FlattenedContract[];
  selectedContractId: Record<string, string>;
  selectedRoleId: Record<string, string>;
  toggleBindingExpanded: (fieldKey: string, fieldIdx: number) => void;
  setSelectedContractId: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  setSelectedRoleId: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  onUpdatePropertyName: (index: number, name: string) => void;
  onUpdatePropertyType: (index: number, type: PropertyType) => void;
  onUpdatePropertySensitivity: (index: number, level: SensitivityLevel) => void;
  onUpdatePropertyOptions: (index: number, options: string[]) => void;
  onRemoveProperty: (index: number) => void;
  onUpdatePropertyContractBindings: (index: number, bindings: ContractRoleBinding[]) => void;
  onContractTypeIdChange: (v: string) => void;
  showDeprecated: boolean;
  fieldUsageMap: Record<string, FieldUsage>;
  onToggleShowDeprecated: () => void;
  onRestoreProperty: (index: number) => void;
  onPermanentlyRemoveProperty: (index: number) => void;
  dynamicGroupEnabled: boolean;
  dynamicGroupAllowedTypes?: PropertyType[];
  dynamicGroupMaxItems?: number;
  dynamicGroupSensitivity?: SensitivityLevel;
  onDynamicGroupEnabledChange: (enabled: boolean) => void;
  onDynamicGroupAllowedTypesChange: (types: PropertyType[]) => void;
  onDynamicGroupMaxItemsChange: (maxItems: number | undefined) => void;
  onDynamicGroupSensitivityChange: (level: SensitivityLevel) => void;
  newFieldType: PropertyType;
  onNewFieldTypeChange: (v: PropertyType) => void;
  onAddProperty: () => void;
}

/**
 * 模板编辑器字段面板：字段行列表（含插件绑定）、已归档字段区、动态字段组开关
 * 与添加字段行。从 TemplateEditor 抽出，保持渲染结构逐字等价。
 */
export function TemplateFieldsPanel({
  editProperties,
  activeRows,
  expandedBindingFields,
  editContractTypeId,
  installedPlugins,
  flattenContracts,
  selectedContractId,
  selectedRoleId,
  toggleBindingExpanded,
  setSelectedContractId,
  setSelectedRoleId,
  onUpdatePropertyName,
  onUpdatePropertyType,
  onUpdatePropertySensitivity,
  onUpdatePropertyOptions,
  onRemoveProperty,
  onUpdatePropertyContractBindings,
  onContractTypeIdChange,
  showDeprecated,
  fieldUsageMap,
  onToggleShowDeprecated,
  onRestoreProperty,
  onPermanentlyRemoveProperty,
  dynamicGroupEnabled,
  dynamicGroupAllowedTypes,
  dynamicGroupMaxItems,
  dynamicGroupSensitivity,
  onDynamicGroupEnabledChange,
  onDynamicGroupAllowedTypesChange,
  onDynamicGroupMaxItemsChange,
  onDynamicGroupSensitivityChange,
  newFieldType,
  onNewFieldTypeChange,
  onAddProperty,
}: TemplateFieldsPanelProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);

  return (
    <div>
      <div
        style={{
          fontSize: 'var(--text-body-sm)',
          fontWeight: 500,
          marginBottom: 8,
          color: 'var(--text-secondary)',
        }}
      >
        {t('settings:fields_section_title', { defaultValue: '字段列表' })}
      </div>

      <div
        style={{
          background: 'var(--bg-toolbar)',
          borderRadius: 8,
          padding: '8px 4px',
          border: '1px solid var(--border-subtle)',
        }}
      >
        {activeRows.length === 0 && editProperties.filter((p) => p.deprecatedAt).length === 0 && (
          <div
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              padding: '12px 0',
            }}
          >
            {t('settings:empty_template_hint', { defaultValue: '此模板暂无字段，点击下方添加' })}
          </div>
        )}

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            maxHeight: '35vh',
            overflowY: 'auto',
            overflowX: 'hidden',
            paddingRight: 4,
          }}
        >
          {activeRows.map(({ prop, idx }) => {
            const fieldKey = prop.id;
            return (
              <TemplateFieldRow
                key={prop.id}
                prop={prop}
                idx={idx}
                isExpanded={expandedBindingFields.has(fieldKey)}
                editContractTypeId={editContractTypeId}
                installedPlugins={installedPlugins}
                flattenContracts={flattenContracts}
                selectedContractId={selectedContractId[fieldKey] || ''}
                selectedRoleId={selectedRoleId[fieldKey] || ''}
                onToggleBindingExpanded={toggleBindingExpanded}
                onSelectedContractChange={(fk, value) => {
                  setSelectedContractId((prev) => ({
                    ...prev,
                    [fk]: value,
                  }));
                  // 切换契约时重置角色选择
                  setSelectedRoleId((prev) => {
                    const next = { ...prev };
                    delete next[fk];
                    return next;
                  });
                }}
                onSelectedRoleChange={(fk, value) => {
                  setSelectedRoleId((prev) => ({
                    ...prev,
                    [fk]: value,
                  }));
                }}
                onUpdatePropertyName={onUpdatePropertyName}
                onUpdatePropertyType={onUpdatePropertyType}
                onUpdatePropertySensitivity={onUpdatePropertySensitivity}
                onUpdatePropertyOptions={onUpdatePropertyOptions}
                onRemoveProperty={onRemoveProperty}
                onUpdatePropertyContractBindings={onUpdatePropertyContractBindings}
                onContractTypeIdChange={onContractTypeIdChange}
              />
            );
          })}
        </div>

        {/* Deprecated fields */}
        {editProperties.filter((p) => p.deprecatedAt).length > 0 && (
          <DeprecatedFieldsSection
            editProperties={editProperties}
            showDeprecated={showDeprecated}
            fieldUsageMap={fieldUsageMap}
            onToggleShowDeprecated={onToggleShowDeprecated}
            onRestoreProperty={onRestoreProperty}
            onPermanentlyRemoveProperty={onPermanentlyRemoveProperty}
          />
        )}
      </div>

      {/* 动态字段组（模板级开关） */}
      <div
        style={{
          marginTop: 10,
          padding: '10px',
          borderRadius: 8,
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-elevated)',
          display: 'flex',
          flexDirection: 'column',
          gap: 10,
        }}
      >
        <label
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            cursor: 'pointer',
            userSelect: 'none',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            color: 'var(--text-primary)',
          }}
        >
          <input
            type="checkbox"
            checked={dynamicGroupEnabled}
            onChange={(e) => onDynamicGroupEnabledChange(e.target.checked)}
            style={{
              width: 16,
              height: 16,
              cursor: 'pointer',
              accentColor: 'var(--accent-primary)',
            }}
          />
          {t('editor:enable_dynamic_group')}
        </label>

        {dynamicGroupEnabled && (
          <DynamicGroupConfig
            allowedTypes={dynamicGroupAllowedTypes}
            maxItems={dynamicGroupMaxItems}
            sensitivity={dynamicGroupSensitivity}
            onAllowedTypesChange={onDynamicGroupAllowedTypesChange}
            onMaxItemsChange={onDynamicGroupMaxItemsChange}
            onSensitivityChange={onDynamicGroupSensitivityChange}
          />
        )}
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 12 }}>
        <TemplateTypeSelect value={newFieldType} onChange={onNewFieldTypeChange} />
        <button
          type="button"
          onClick={onAddProperty}
          className="interactive-toolbar"
          style={{
            height: 36,
            padding: '0 14px',
            borderRadius: 6,
            borderWidth: 1,
            borderStyle: 'solid',
            fontSize: 'var(--text-body-sm)',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            whiteSpace: 'nowrap',
          }}
        >
          <Plus size={ICON_SIZE.sm} />
          {t('settings:add_field', { defaultValue: '添加字段' })}
        </button>
      </div>
    </div>
  );
}
