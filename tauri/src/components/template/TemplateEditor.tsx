import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, ChevronRight } from 'lucide-react';
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
import { TemplateTypeSelect } from './TemplateTypeSelect';
import { TemplatePageSelect } from './TemplatePageSelect';
import { IconPicker } from './IconPicker';
import { DynamicGroupConfig } from './DynamicGroupConfig';
import { ICON_SIZE } from '@/lib/constants';
import { usePluginStore } from '@/stores/pluginStore';
import { resolvePluginName, deriveContractBindings } from '@/lib/plugin';
import { TemplateFieldRow, type FlattenedContract } from './TemplateFieldRow';
import { DeprecatedFieldsSection, type FieldUsage } from './DeprecatedFieldsSection';

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
 * 字段行/插件绑定 → TemplateFieldRow；已归档字段 → DeprecatedFieldsSection。
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
  const [showIconPicker, setShowIconPicker] = useState(false);

  // 插件绑定 UI 状态
  const [expandedBindingFields, setExpandedBindingFields] = useState<Set<string>>(new Set());
  const [selectedContractId, setSelectedContractId] = useState<Record<string, string>>({});
  const [selectedRoleId, setSelectedRoleId] = useState<Record<string, string>>({});

  const installedPlugins = usePluginStore((s) => s.installedPlugins);
  const loadInstalled = usePluginStore((s) => s.loadInstalled);
  const { t, i18n } = useTranslation(['settings', 'common', 'editor']);

  // 加载已安装插件列表（用于展示契约角色）
  React.useEffect(() => {
    if (installedPlugins.length === 0) {
      loadInstalled().catch(() => {});
    }
  }, [installedPlugins.length, loadInstalled]);

  // 将已安装插件的所有契约展平为一个列表
  const flattenContracts = React.useMemo<FlattenedContract[]>(() => {
    const currentLocale = i18n.language || 'zh-CN';
    const list: FlattenedContract[] = [];
    for (const plugin of installedPlugins) {
      for (const contract of plugin.contracts || []) {
        if (contract.roles && contract.roles.length > 0) {
          list.push({
            pluginId: plugin.id,
            pluginName: resolvePluginName(plugin, currentLocale),
            contract,
          });
        }
      }
    }
    return list;
  }, [installedPlugins, i18n.language]);

  const toggleBindingExpanded = (fieldKey: string, fieldIdx: number) => {
    const willExpand = !expandedBindingFields.has(fieldKey);
    setExpandedBindingFields((prev) => {
      const next = new Set(prev);
      if (next.has(fieldKey)) {
        next.delete(fieldKey);
      } else {
        next.add(fieldKey);
      }
      return next;
    });
    // 展开时自动推导并持久化 contractField: true 但无硬编码 bindings 的字段
    if (willExpand) {
      const prop = editProperties[fieldIdx];
      if (prop) {
        const existingBindings = prop.contractBindings || [];
        if (existingBindings.length === 0 && prop.contractField && editContractTypeId) {
          const derived = deriveContractBindings(editContractTypeId, prop.id, installedPlugins);
          if (derived.length > 0) {
            onUpdatePropertyContractBindings(fieldIdx, derived);
          }
        }
      }
    }
  };

  const activeRows = editProperties
    .map((prop, idx) => ({ prop, idx }))
    .filter(({ prop }) => !prop.deprecatedAt && prop.type !== 'dynamic_group');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
      <div className={nameError ? 'name-input-error' : ''}>
        <Input
          label={t('common:name') || '名称'}
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
        label={t('settings:template_category') || '所属页面'}
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
            {t('settings:contract_type_id') || '契约类型'}:
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
          <span style={{ flex: 1 }}>{t('settings:template_icon') || '模板图标'}</span>
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {showIconPicker
              ? t('common:collapse') || '收起'
              : t('settings:click_to_change_icon') || '点击选择图标'}
          </span>
        </button>
        {showIconPicker && <IconPicker value={editIconId} onChange={onEditIconIdChange} />}
      </div>

      <div>
        <div
          style={{
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            marginBottom: 8,
            color: 'var(--text-secondary)',
          }}
        >
          {t('settings:fields_section_title') || '字段列表'}
        </div>

        <div
          style={{
            background: 'var(--bg-toolbar)',
            borderRadius: 8,
            padding: '8px 4px',
            border: '1px solid var(--border-subtle)',
          }}
        >
          {activeRows.length === 0 &&
            editProperties.filter((p) => p.deprecatedAt).length === 0 && (
              <div
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  padding: '12px 0',
                }}
              >
                {t('settings:empty_template_hint') || '此模板暂无字段，点击下方添加'}
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
            onMouseEnter={(e) => {
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'var(--bg-toolbar)';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
            }}
            style={{
              height: 36,
              padding: '0 14px',
              borderRadius: 6,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-toolbar)',
              color: 'var(--text-primary)',
              fontSize: 'var(--text-body-sm)',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              whiteSpace: 'nowrap',
              transition: 'background 0.2s, border-color 0.2s',
            }}
          >
            <Plus size={ICON_SIZE.sm} />
            {t('settings:add_field') || '添加字段'}
          </button>
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
        <Button variant="secondary" onClick={onClose}>
          {t('common:cancel') || '取消'}
        </Button>
        <Button variant="secondary" onClick={onSave}>
          {t('common:save') || '保存'}
        </Button>
      </div>
    </div>
  );
}
