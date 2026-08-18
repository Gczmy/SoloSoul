import React from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { CUSTOM_ICON_MAP, resolveCustomIcon, type CustomIconId } from '@/lib/pageIcons';
import type {
  UserTemplate,
  TemplateProperty,
  PropertyType,
  SensitivityLevel,
  ContractRoleBinding,
} from '@/types/template';
import { TemplatePageSelect } from './TemplatePageSelect';
import { IconPicker } from './IconPicker';
import { TemplateFieldsPanel } from './TemplateFieldsPanel';
import { ICON_SIZE } from '@/lib/constants';
import type { FieldUsage } from './DeprecatedFieldsSection';
import { useTemplateEditorState } from './useTemplateEditorState';

interface TemplateEditorProps {
  editingTemplate: UserTemplate | null;

  editName: string;
  editCategory: string;
  editIconId: string;
  editContractTypeId: string;
  editProperties: TemplateProperty[];
  newFieldType: PropertyType;
  showDeprecated: boolean;
  fieldUsageMap: Record<string, FieldUsage>;
  onEditNameChange: (v: string) => void;
  onEditCategoryChange: (v: string) => void;
  onEditIconIdChange: (v: string) => void;
  onContractTypeIdChange: (v: string) => void;
  onNewFieldTypeChange: (v: PropertyType) => void;
  onAddProperty: () => void;
  onUpdatePropertyName: (index: number, name: string) => void;
  onUpdatePropertyType: (index: number, type: PropertyType) => void;
  onUpdatePropertySensitivity: (index: number, level: SensitivityLevel) => void;
  onUpdatePropertyOptions: (index: number, options: string[]) => void;
  onRemoveProperty: (index: number) => void;
  onUpdatePropertyContractBindings: (index: number, bindings: ContractRoleBinding[]) => void;
  onRestoreProperty: (index: number) => void;
  onPermanentlyRemoveProperty: (index: number) => void;
  onToggleShowDeprecated: () => void;
  onSave: () => void;
  onClose: () => void;

  // 名称输入错误提示
  nameError?: boolean;

  // 动态字段组（模板级开关）
  dynamicGroupEnabled: boolean;
  dynamicGroupAllowedTypes?: PropertyType[];
  dynamicGroupMaxItems?: number;
  dynamicGroupSensitivity?: SensitivityLevel;
  onDynamicGroupEnabledChange: (enabled: boolean) => void;
  onDynamicGroupAllowedTypesChange: (types: PropertyType[]) => void;
  onDynamicGroupMaxItemsChange: (maxItems: number | undefined) => void;
  onDynamicGroupSensitivityChange: (level: SensitivityLevel) => void;
}

/**
 * 模板编辑器：编排层。
 * 本地状态/插件契约 → useTemplateEditorState；字段行/插件绑定 → TemplateFieldsPanel
 * （内部 TemplateFieldRow）；已归档字段 → DeprecatedFieldsSection。
 */
export function TemplateEditor({
  editName,
  editCategory,
  editIconId,
  editContractTypeId,
  editProperties,
  newFieldType,
  showDeprecated,
  fieldUsageMap,
  onEditNameChange,
  onEditCategoryChange,
  onEditIconIdChange,
  onNewFieldTypeChange,
  onAddProperty,
  onUpdatePropertyName,
  onUpdatePropertyType,
  onUpdatePropertySensitivity,
  onUpdatePropertyOptions,
  onRemoveProperty,
  onContractTypeIdChange,
  onUpdatePropertyContractBindings,
  onRestoreProperty,
  onPermanentlyRemoveProperty,
  onToggleShowDeprecated,
  onSave,
  onClose,
  nameError,
  dynamicGroupEnabled,
  dynamicGroupAllowedTypes,
  dynamicGroupMaxItems,
  dynamicGroupSensitivity,
  onDynamicGroupEnabledChange,
  onDynamicGroupAllowedTypesChange,
  onDynamicGroupMaxItemsChange,
  onDynamicGroupSensitivityChange,
}: TemplateEditorProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);
  const {
    showIconPicker,
    setShowIconPicker,
    expandedBindingFields,
    selectedContractId,
    setSelectedContractId,
    selectedRoleId,
    setSelectedRoleId,
    installedPlugins,
    flattenContracts,
    toggleBindingExpanded,
  } = useTemplateEditorState(editProperties, editContractTypeId, onUpdatePropertyContractBindings);

  const activeRows = editProperties
    .map((prop, idx) => ({ prop, idx }))
    .filter(({ prop }) => !prop.deprecatedAt && prop.type !== 'dynamic_group');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
      <div className={nameError ? 'name-input-error' : ''}>
        <Input
          label={t('common:name', { defaultValue: '名称' })}
          value={editName}
          onChange={(e) => {
            onEditNameChange(e.target.value);
          }}
        />
      </div>
      {nameError && (
        <style>{`
          @keyframes nameInputShake {
            0%, 100% { transform: translateX(0); }
            20% { transform: translateX(-6px); }
            40% { transform: translateX(6px); }
            60% { transform: translateX(-4px); }
            80% { transform: translateX(4px); }
          }
          .name-input-error input {
            border-color: #ef4444 !important;
            animation: nameInputShake 0.4s ease-in-out;
          }
        `}</style>
      )}
      <TemplatePageSelect
        value={editCategory}
        onChange={onEditCategoryChange}
        label={t('settings:template_category', { defaultValue: '所属页面' })}
      />

      {/* 插件契约类型 ID（自动绑定到字段时设置） */}
      {editContractTypeId && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '6px 10px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'color-mix(in srgb, var(--accent-primary) 5%, transparent)',
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
          }}
        >
          <span style={{ fontWeight: 500, whiteSpace: 'nowrap' }}>
            {t('settings:contract_type_id', { defaultValue: '契约类型' })}:
          </span>
          <span style={{ fontFamily: 'monospace', fontSize: 'var(--text-badge)' }}>
            {editContractTypeId}
          </span>
        </div>
      )}

      {/* Icon picker — collapsible, default collapsed */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        <button
          type="button"
          onClick={() => setShowIconPicker((v) => !v)}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '6px 10px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-toolbar)',
            cursor: 'pointer',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            color: 'var(--text-secondary)',
            fontFamily: 'inherit',
            textAlign: 'left',
            width: '100%',
          }}
        >
          <span
            style={{
              transform: showIconPicker ? 'rotate(90deg)' : 'rotate(0deg)',
              transition: 'transform 0.15s ease',
              display: 'inline-flex',
              flexShrink: 0,
            }}
          >
            <ChevronRight size={ICON_SIZE.sm} />
          </span>
          {React.createElement(
            editIconId && editIconId in CUSTOM_ICON_MAP
              ? CUSTOM_ICON_MAP[editIconId as CustomIconId]
              : resolveCustomIcon(editIconId),
            { size: ICON_SIZE.lg, style: { color: 'var(--accent-primary)', flexShrink: 0 } },
          )}
          <span style={{ flex: 1 }}>
            {t('settings:template_icon', { defaultValue: '模板图标' })}
          </span>
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {showIconPicker
              ? t('common:collapse', { defaultValue: '收起' })
              : t('settings:click_to_change_icon', { defaultValue: '点击选择图标' })}
          </span>
        </button>
        {showIconPicker && <IconPicker value={editIconId} onChange={onEditIconIdChange} />}
      </div>

      <TemplateFieldsPanel
        editProperties={editProperties}
        activeRows={activeRows}
        expandedBindingFields={expandedBindingFields}
        editContractTypeId={editContractTypeId}
        installedPlugins={installedPlugins}
        flattenContracts={flattenContracts}
        selectedContractId={selectedContractId}
        selectedRoleId={selectedRoleId}
        toggleBindingExpanded={toggleBindingExpanded}
        setSelectedContractId={setSelectedContractId}
        setSelectedRoleId={setSelectedRoleId}
        onUpdatePropertyName={onUpdatePropertyName}
        onUpdatePropertyType={onUpdatePropertyType}
        onUpdatePropertySensitivity={onUpdatePropertySensitivity}
        onUpdatePropertyOptions={onUpdatePropertyOptions}
        onRemoveProperty={onRemoveProperty}
        onUpdatePropertyContractBindings={onUpdatePropertyContractBindings}
        onContractTypeIdChange={onContractTypeIdChange}
        showDeprecated={showDeprecated}
        fieldUsageMap={fieldUsageMap}
        onToggleShowDeprecated={onToggleShowDeprecated}
        onRestoreProperty={onRestoreProperty}
        onPermanentlyRemoveProperty={onPermanentlyRemoveProperty}
        dynamicGroupEnabled={dynamicGroupEnabled}
        dynamicGroupAllowedTypes={dynamicGroupAllowedTypes}
        dynamicGroupMaxItems={dynamicGroupMaxItems}
        dynamicGroupSensitivity={dynamicGroupSensitivity}
        onDynamicGroupEnabledChange={onDynamicGroupEnabledChange}
        onDynamicGroupAllowedTypesChange={onDynamicGroupAllowedTypesChange}
        onDynamicGroupMaxItemsChange={onDynamicGroupMaxItemsChange}
        onDynamicGroupSensitivityChange={onDynamicGroupSensitivityChange}
        newFieldType={newFieldType}
        onNewFieldTypeChange={onNewFieldTypeChange}
        onAddProperty={onAddProperty}
      />

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
        <Button variant="secondary" onClick={onClose}>
          {t('common:cancel', { defaultValue: '取消' })}
        </Button>
        <Button variant="primary" onClick={onSave}>
          {t('common:save', { defaultValue: '保存' })}
        </Button>
      </div>
    </div>
  );
}
