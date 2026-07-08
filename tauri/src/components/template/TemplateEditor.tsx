import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Plus, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SensitivityBadge as UiSensitivityBadge } from '@/components/ui/SensitivityBadge';
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
import { OptionsEditor } from './OptionsEditor';
import { DynamicGroupConfig } from './DynamicGroupConfig';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { ICON_SIZE } from '@/lib/constants';
import { usePluginStore } from '@/stores/pluginStore';
import {
  resolvePluginName,
  deriveContractBindings,
  type PluginContractBinding as PluginContractBindingType,
} from '@/lib/plugin';

interface FieldUsage {
  active: number;
  softDeleted: number;
}

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
  onUpdatePropertyAllowedTypes: (index: number, types: PropertyType[]) => void;
  onUpdatePropertyMaxItems: (index: number, maxItems: number | undefined) => void;
  onRemoveProperty: (index: number) => void;
  onUpdatePropertyContractBindings: (index: number, bindings: ContractRoleBinding[]) => void;
  onRestoreProperty: (index: number) => void;
  onPermanentlyRemoveProperty: (index: number) => void;
  onToggleShowDeprecated: () => void;
  onSave: () => void;
  onClose: () => void;
}

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

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
  onUpdatePropertyAllowedTypes,
  onUpdatePropertyMaxItems,
  onRemoveProperty,
  onContractTypeIdChange,
  onUpdatePropertyContractBindings,
  onRestoreProperty,
  onPermanentlyRemoveProperty,
  onToggleShowDeprecated,
  onSave,
  onClose,
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
  const flattenContracts = React.useMemo(() => {
    const currentLocale = i18n.language || 'zh-CN';
    const list: Array<{
      pluginId: string;
      pluginName: string;
      contract: PluginContractBindingType;
    }> = [];
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

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
      <Input
        label={t('common:name') || '名称'}
        value={editName}
        onChange={(e) => onEditNameChange(e.target.value)}
      />
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
            { size: 18, style: { color: 'var(--accent-primary)', flexShrink: 0 } },
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
          {editProperties.filter((p) => !p.deprecatedAt).length === 0 &&
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
            {editProperties
              .map((prop, idx) => ({ prop, idx }))
              .filter(({ prop }) => !prop.deprecatedAt)
              .map(({ prop, idx }) => {
                const bindings = prop.contractBindings || [];
                const fieldKey = prop.id;
                const isExpanded = expandedBindingFields.has(fieldKey);

                // 自动推导：contractField: true 但无硬编码 bindings 时，从已安装插件 manifest 匹配
                const derivedBindings = (bindings.length === 0 && prop.contractField && editContractTypeId)
                  ? deriveContractBindings(editContractTypeId, prop.id, installedPlugins)
                  : [];
                const effectiveBindings = bindings.length > 0 ? bindings : derivedBindings;
                const currentContractId = selectedContractId[fieldKey] || '';
                const currentRoleId = selectedRoleId[fieldKey] || '';

                // 查找当前选中的契约
                const selectedFlat = flattenContracts.find(
                  (fc) => `${fc.pluginId}::${fc.contract.typeId}` === currentContractId,
                );
                // 当前契约的可用角色
                const availableRoles = selectedFlat?.contract.roles || [];

                const handleAddBinding = () => {
                  if (!currentContractId || !currentRoleId) return;
                  const [, contractTypeId] = currentContractId.split('::');
                  // 去重检查
                  const exists = bindings.some(
                    (b) => b.contractTypeId === contractTypeId && b.roleId === currentRoleId,
                  );
                  if (exists) return;
                  const newBindings = [...bindings, { contractTypeId, roleId: currentRoleId }];
                  onUpdatePropertyContractBindings(idx, newBindings);
                  // 自动设置模板 contractTypeId（首次或不同时更新）
                  if (editContractTypeId !== contractTypeId) {
                    onContractTypeIdChange(contractTypeId);
                  }
                };

                const handleRemoveBinding = (contractTypeId: string, roleId: string) => {
                  const newBindings = bindings.filter(
                    (b) => !(b.contractTypeId === contractTypeId && b.roleId === roleId),
                  );
                  onUpdatePropertyContractBindings(idx, newBindings);
                };

                // 根据 contractTypeId 查找插件名称、displayName
                const getContractInfo = (ctid: string) => {
                  const fc = flattenContracts.find((x) => x.contract.typeId === ctid);
                  return fc
                    ? {
                        pluginName: fc.pluginName,
                        displayName: fc.contract.displayName || ctid,
                      }
                    : { pluginName: ctid, displayName: ctid };
                };

                // 根据 contractTypeId + roleId 查找角色标签和 required 标记
                const getRoleInfo = (ctid: string, roleId: string) => {
                  const fc = flattenContracts.find((x) => x.contract.typeId === ctid);
                  if (!fc) return { label: roleId, required: false };
                  const role = fc.contract.roles.find((r) => r.roleId === roleId);
                  return {
                    label: role?.label || roleId,
                    required: role?.required || false,
                  };
                };

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
                          placeholder={t('settings:field_name') || '字段名称'}
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
                            'dynamic_group',
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
                      {prop.type === 'dynamic_group' && (
                        <DynamicGroupConfig
                          allowedTypes={prop.allowedTypes}
                          maxItems={prop.maxItems}
                          onAllowedTypesChange={(types) => onUpdatePropertyAllowedTypes(idx, types)}
                          onMaxItemsChange={(max) => onUpdatePropertyMaxItems(idx, max)}
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
                        title={t('settings:remove_field') || '删除'}
                        iconOnly
                      />
                    </div>

                    {/* 插件绑定折叠区域 */}
                    <div style={{ paddingLeft: 10, marginTop: 2, marginBottom: 6 }}>
                      <button
                        type="button"
                        onClick={() => toggleBindingExpanded(fieldKey, idx)}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 6,
                          padding: '4px 8px',
                          borderRadius: 6,
                          border: '1px solid transparent',
                          background: 'transparent',
                          cursor: 'pointer',
                          fontSize: 'var(--text-body-sm)',
                          fontWeight: 500,
                          color: 'var(--text-secondary)',
                          fontFamily: 'inherit',
                          textAlign: 'left',
                          width: '100%',
                          transition: 'background 0.15s',
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.background = 'var(--bg-toolbar)';
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.background = 'transparent';
                        }}
                      >
                        <span
                          style={{
                            transform: isExpanded ? 'rotate(90deg)' : 'rotate(0deg)',
                            transition: 'transform 0.15s ease',
                            display: 'inline-flex',
                            flexShrink: 0,
                          }}
                        >
                          <ChevronRight size={14} />
                        </span>
                        {t('settings:plugin_binding') || '插件绑定'}
                        {effectiveBindings.length > 0 && (
                          <span
                            style={{
                              fontSize: 'var(--text-badge)',
                              color: 'var(--accent-primary)',
                              marginLeft: 4,
                            }}
                          >
                            ({effectiveBindings.length})
                          </span>
                        )}
                        <span style={{ flex: 1 }} />
                        {effectiveBindings.length > 0 || flattenContracts.length > 0 ? (
                          <span
                            style={{
                              fontSize: 'var(--text-caption)',
                              color: 'var(--text-tertiary)',
                            }}
                          >
                            {isExpanded
                              ? t('common:collapse') || '收起'
                              : t('settings:click_to_configure') || '点击配置'}
                          </span>
                        ) : null}
                      </button>

                      {isExpanded && (
                        <div
                          style={{
                            padding: '8px 8px 8px 24px',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: 8,
                          }}
                        >
                          {/* 已绑定标签列表 */}
                          {effectiveBindings.length > 0 && (
                            <div
                              style={{
                                display: 'flex',
                                flexWrap: 'wrap',
                                gap: 6,
                                marginBottom: 4,
                              }}
                            >
                              {effectiveBindings.map((b) => {
                                const ci = getContractInfo(b.contractTypeId);
                                const ri = getRoleInfo(b.contractTypeId, b.roleId);
                                const isDerived = bindings.length === 0;
                                return (
                                  <span
                                    key={`${b.contractTypeId}::${b.roleId}`}
                                    style={{
                                      display: 'inline-flex',
                                      alignItems: 'center',
                                      gap: 4,
                                      padding: '2px 8px',
                                      borderRadius: 4,
                                      fontSize: 'var(--text-badge)',
                                      background: isDerived
                                        ? 'color-mix(in srgb, var(--accent-primary) 4%, transparent)'
                                        : 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                                      color: 'var(--accent-primary)',
                                      border: isDerived
                                        ? '1px dashed color-mix(in srgb, var(--accent-primary) 30%, transparent)'
                                        : '1px solid color-mix(in srgb, var(--accent-primary) 20%, transparent)',
                                    }}
                                  >
                                    <span style={{ fontWeight: 500 }}>{ci.pluginName}</span>
                                    <span style={{ opacity: 0.5 }}>—</span>
                                    <span>{ci.displayName}</span>
                                    <span style={{ opacity: 0.6 }}>/</span>
                                    <span>{ri.label}</span>
                                    {ri.required && (
                                      <span
                                        style={{
                                          color: 'var(--warning)',
                                          fontWeight: 600,
                                          fontSize: 11,
                                        }}
                                      >
                                        *
                                      </span>
                                    )}
                                    {!isDerived && (
                                      <button
                                        type="button"
                                        onClick={() =>
                                          handleRemoveBinding(b.contractTypeId, b.roleId)
                                        }
                                        style={{
                                          background: 'none',
                                          border: 'none',
                                          cursor: 'pointer',
                                          padding: '0 2px',
                                          color: 'var(--accent-primary)',
                                          fontSize: 14,
                                          lineHeight: 1,
                                          opacity: 0.7,
                                          transition: 'opacity 0.15s',
                                        }}
                                        onMouseEnter={(e) => {
                                          e.currentTarget.style.opacity = '1';
                                        }}
                                        onMouseLeave={(e) => {
                                          e.currentTarget.style.opacity = '0.7';
                                        }}
                                        title={t('common:remove') || '移除'}
                                      >
                                        ✕
                                      </button>
                                    )}
                                  </span>
                                );
                              })}
                            </div>
                          )}

                          {/* 添加绑定 */}
                          {flattenContracts.length > 0 ? (
                            <div
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: 6,
                                flexWrap: 'wrap',
                              }}
                            >
                              <select
                                value={currentContractId}
                                onChange={(e) => {
                                  setSelectedContractId((prev) => ({
                                    ...prev,
                                    [fieldKey]: e.target.value,
                                  }));
                                  // 切换契约时重置角色选择
                                  setSelectedRoleId((prev) => {
                                    const next = { ...prev };
                                    delete next[fieldKey];
                                    return next;
                                  });
                                }}
                                style={{
                                  height: 32,
                                  padding: '0 8px',
                                  borderRadius: 6,
                                  border: '1px solid var(--border-subtle)',
                                  background: 'var(--bg-elevated)',
                                  color: 'var(--text-primary)',
                                  fontSize: 'var(--text-body-sm)',
                                  cursor: 'pointer',
                                  boxSizing: 'border-box',
                                  minWidth: 180,
                                  flex: 1,
                                }}
                              >
                                <option value="">
                                  {t('settings:select_plugin_contract') || '选择插件契约'}
                                </option>
                                {flattenContracts.map((fc) => {
                                  const val = `${fc.pluginId}::${fc.contract.typeId}`;
                                  const displayName = fc.contract.displayName || fc.contract.typeId;
                                  return (
                                    <option key={val} value={val}>
                                      {fc.pluginName} — {displayName}
                                    </option>
                                  );
                                })}
                              </select>

                              <select
                                value={currentRoleId}
                                onChange={(e) => {
                                  setSelectedRoleId((prev) => ({
                                    ...prev,
                                    [fieldKey]: e.target.value,
                                  }));
                                }}
                                disabled={!currentContractId}
                                style={{
                                  height: 32,
                                  padding: '0 8px',
                                  borderRadius: 6,
                                  border: '1px solid var(--border-subtle)',
                                  background: 'var(--bg-elevated)',
                                  color: 'var(--text-primary)',
                                  fontSize: 'var(--text-body-sm)',
                                  cursor: currentContractId ? 'pointer' : 'not-allowed',
                                  boxSizing: 'border-box',
                                  minWidth: 120,
                                  flex: 1,
                                  opacity: currentContractId ? 1 : 0.5,
                                }}
                              >
                                <option value="">{t('settings:select_role') || '选择角色'}</option>
                                {availableRoles.map((role) => (
                                  <option key={role.roleId} value={role.roleId}>
                                    {role.label || role.roleId}
                                    {role.required ? ' *' : ''}
                                  </option>
                                ))}
                              </select>

                              <button
                                type="button"
                                onClick={handleAddBinding}
                                disabled={!currentContractId || !currentRoleId}
                                style={{
                                  height: 32,
                                  padding: '0 12px',
                                  borderRadius: 6,
                                  border: '1px solid var(--accent-primary)',
                                  background:
                                    !currentContractId || !currentRoleId
                                      ? 'var(--bg-toolbar)'
                                      : 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                                  color:
                                    !currentContractId || !currentRoleId
                                      ? 'var(--text-tertiary)'
                                      : 'var(--accent-primary)',
                                  fontSize: 'var(--text-body-sm)',
                                  cursor:
                                    !currentContractId || !currentRoleId
                                      ? 'not-allowed'
                                      : 'pointer',
                                  whiteSpace: 'nowrap',
                                  transition: 'background 0.15s',
                                }}
                              >
                                {t('common:add') || 'Add'}
                              </button>
                            </div>
                          ) : (
                            <div
                              style={{
                                fontSize: 'var(--text-caption)',
                                color: 'var(--text-tertiary)',
                                padding: '4px 0',
                              }}
                            >
                              {t('settings:no_plugin_contracts_available') ||
                                '暂无已安装的插件契约（需安装含有角色定义的插件）'}
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  </React.Fragment>
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

interface DeprecatedFieldsSectionProps {
  editProperties: TemplateProperty[];
  showDeprecated: boolean;
  fieldUsageMap: Record<string, FieldUsage>;
  onToggleShowDeprecated: () => void;
  onRestoreProperty: (index: number) => void;
  onPermanentlyRemoveProperty: (index: number) => void;
}

function DeprecatedFieldsSection({
  editProperties,
  showDeprecated,
  fieldUsageMap,
  onToggleShowDeprecated,
  onRestoreProperty,
  onPermanentlyRemoveProperty,
}: DeprecatedFieldsSectionProps) {
  const { t } = useTranslation(['settings', 'common']);

  const deprecatedFields = editProperties
    .map((prop, idx) => ({ prop, idx }))
    .filter(({ prop }) => prop.deprecatedAt);

  return (
    <div style={{ marginTop: 16 }}>
      <button
        onClick={onToggleShowDeprecated}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          fontSize: 'var(--text-caption)',
          color: 'var(--text-secondary)',
          background: 'transparent',
          border: 'none',
          cursor: 'pointer',
          padding: 0,
          fontWeight: 500,
        }}
      >
        <span
          style={{
            transform: showDeprecated ? 'rotate(90deg)' : 'rotate(0deg)',
            transition: 'transform 0.15s ease',
            display: 'inline-block',
          }}
        >
          ▶
        </span>
        {t('settings:deprecated_fields_count', {
          count: deprecatedFields.length,
        })}
      </button>
      {showDeprecated && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            marginTop: 8,
            maxHeight: '20vh',
            overflowY: 'auto',
            overflowX: 'hidden',
            paddingRight: 4,
          }}
        >
          {deprecatedFields.map(({ prop, idx }) => {
            const usage = fieldUsageMap[prop.id];
            const cleanable = usage ? usage.active === 0 && usage.softDeleted === 0 : false;
            return (
              <div
                key={prop.id}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 6,
                  padding: '8px 10px',
                  borderRadius: 6,
                  background: 'var(--bg-toolbar)',
                  border: '1px solid var(--border-subtle)',
                  opacity: 0.75,
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <FieldTypeIcon type={prop.type} size={ICON_SIZE.sm} />
                  <span
                    style={{
                      fontSize: 'var(--text-body)',
                      fontWeight: 500,
                      color: 'var(--text-secondary)',
                      flex: 1,
                      minWidth: 1,
                      textDecoration: 'line-through',
                    }}
                  >
                    {prop.name}
                  </span>
                  <UiSensitivityBadge
                    level={(prop.sensitivityLevel || 'internal') as SensitivityLevel}
                  />
                  <DeprecatedBadge />
                  <Button variant="tertiary" size="sm" onClick={() => onRestoreProperty(idx)}>
                    {t('common:restore') || '恢复'}
                  </Button>
                  {cleanable && (
                    <Button
                      variant="tertiary"
                      size="sm"
                      onClick={() => onPermanentlyRemoveProperty(idx)}
                      style={{ color: '#e74c3c' }}
                    >
                      {t('common:clean_up') || '清理'}
                    </Button>
                  )}
                </div>
                {usage && !cleanable && (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      justifyContent: 'space-between',
                    }}
                  >
                    <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {usage.active > 0 && usage.softDeleted > 0
                        ? t('settings:field_in_use_both', {
                            activeCount: usage.active,
                            softDeletedCount: usage.softDeleted,
                          })
                        : usage.active > 0
                          ? t('settings:field_in_use_active', {
                              activeCount: usage.active,
                            })
                          : t('settings:field_in_use_trash', {
                              softDeletedCount: usage.softDeleted,
                            })}
                    </span>
                    {usage.softDeleted > 0 && (
                      <span
                        onClick={() => {}}
                        style={{
                          fontSize: 'var(--text-badge)',
                          color: 'var(--accent-primary)',
                          cursor: 'pointer',
                          textDecoration: 'underline',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {t('common:go_to_trash') || '前往回收站'}
                      </span>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
