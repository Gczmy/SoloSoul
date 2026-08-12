import React from 'react';
import { useTranslation } from 'react-i18next';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { OptionsEditor } from './OptionsEditor';
import { TemplateFieldBindingSection } from './TemplateFieldBindingSection';
import type { FlattenedContract } from './TemplateFieldBindingSection';
import type {
  ContractRoleBinding,
  PropertyType,
  SensitivityLevel,
  TemplateProperty,
} from '@/types/template';
import type { PluginManifest } from '@/lib/plugin';

export type { FlattenedContract } from './TemplateFieldBindingSection';

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

interface TemplateFieldRowProps {
  prop: TemplateProperty;
  idx: number;
  isExpanded: boolean;
  editContractTypeId: string;
  installedPlugins: PluginManifest[];
  flattenContracts: FlattenedContract[];
  selectedContractId: string;
  selectedRoleId: string;
  onToggleBindingExpanded: (fieldKey: string, fieldIdx: number) => void;
  onSelectedContractChange: (fieldKey: string, value: string) => void;
  onSelectedRoleChange: (fieldKey: string, value: string) => void;
  onUpdatePropertyName: (index: number, name: string) => void;
  onUpdatePropertyType: (index: number, type: PropertyType) => void;
  onUpdatePropertySensitivity: (index: number, level: SensitivityLevel) => void;
  onUpdatePropertyOptions: (index: number, options: string[]) => void;
  onRemoveProperty: (index: number) => void;
  onUpdatePropertyContractBindings: (index: number, bindings: ContractRoleBinding[]) => void;
  onContractTypeIdChange: (v: string) => void;
}

/**
 * 模板字段行：名称/类型/敏感度/选项编辑 + 插件契约绑定折叠区。
 * P046 拆分后：顶部基础控件行保留于此；插件绑定折叠区（约 270 行内联渲染
 * 与派生/增删逻辑）收敛于 TemplateFieldBindingSection 子组件。
 */
export function TemplateFieldRow({
  prop,
  idx,
  isExpanded,
  editContractTypeId,
  installedPlugins,
  flattenContracts,
  selectedContractId,
  selectedRoleId,
  onToggleBindingExpanded,
  onSelectedContractChange,
  onSelectedRoleChange,
  onUpdatePropertyName,
  onUpdatePropertyType,
  onUpdatePropertySensitivity,
  onUpdatePropertyOptions,
  onRemoveProperty,
  onUpdatePropertyContractBindings,
  onContractTypeIdChange,
}: TemplateFieldRowProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);

  return (
    <React.Fragment key={prop.id}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '8px 10px',
          borderRadius: 6,
          background: 'var(--bg-subtle)',
        }}
      >
        <div style={{ flex: 1, minWidth: 80 }}>
          <input
            value={prop.name}
            onChange={(e) => onUpdatePropertyName(idx, e.target.value)}
            placeholder={t('settings:field_name', { defaultValue: '字段名称' })}
            style={{
              width: '100%',
              height: 36,
              padding: '0 10px',
              borderRadius: 6,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontSize: 'var(--text-body)',
              fontFamily: 'inherit',
              outline: 'none',
              boxSizing: 'border-box',
            }}
          />
        </div>
        <select
          value={prop.type}
          onChange={(e) => onUpdatePropertyType(idx, e.target.value as PropertyType)}
          style={{
            height: 36,
            padding: '0 10px',
            borderRadius: 6,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            fontSize: 'var(--text-body-sm)',
            cursor: 'pointer',
            boxSizing: 'border-box',
            minWidth: 90,
          }}
        >
          {(
            [
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
            ] as PropertyType[]
          ).map((pt) => (
            <option key={pt} value={pt}>
              {t(`editor:field_types.${pt}`, pt)}
            </option>
          ))}
        </select>
        {(prop.type === 'select' || prop.type === 'multiselect') && (
          <OptionsEditor
            options={prop.options || []}
            onChange={(opts) => onUpdatePropertyOptions(idx, opts)}
            fieldName={prop.name}
            fieldType={prop.type === 'multiselect' ? 'multiselect' : 'select'}
          />
        )}
        <select
          value={prop.sensitivityLevel || 'internal'}
          onChange={(e) =>
            onUpdatePropertySensitivity(idx, e.target.value as SensitivityLevel)
          }
          style={{
            height: 36,
            padding: '0 10px',
            borderRadius: 6,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            fontSize: 'var(--text-body-sm)',
            cursor: 'pointer',
            boxSizing: 'border-box',
          }}
        >
          {SENSITIVITY_LEVELS.map((sl) => (
            <option key={sl} value={sl}>
              {t(`editor:sensitivity_levels.${sl}`, sl)}
            </option>
          ))}
        </select>
        <DeleteButton
          onClick={() => onRemoveProperty(idx)}
          title={t('settings:remove_field', { defaultValue: '删除' })}
          iconOnly
        />
      </div>

      {/* 插件绑定折叠区域（P046 拆分：TemplateFieldBindingSection） */}
      <TemplateFieldBindingSection
        prop={prop}
        idx={idx}
        isExpanded={isExpanded}
        editContractTypeId={editContractTypeId}
        installedPlugins={installedPlugins}
        flattenContracts={flattenContracts}
        selectedContractId={selectedContractId}
        selectedRoleId={selectedRoleId}
        onToggleBindingExpanded={onToggleBindingExpanded}
        onSelectedContractChange={onSelectedContractChange}
        onSelectedRoleChange={onSelectedRoleChange}
        onUpdatePropertyContractBindings={onUpdatePropertyContractBindings}
        onContractTypeIdChange={onContractTypeIdChange}
      />
    </React.Fragment>
  );
}
