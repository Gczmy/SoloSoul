import React from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight } from 'lucide-react';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { OptionsEditor } from './OptionsEditor';
import {
  deriveContractBindings,
  type PluginContractBinding as PluginContractBindingType,
  type PluginManifest,
} from '@/lib/plugin';
import type {
  ContractRoleBinding,
  PropertyType,
  SensitivityLevel,
  TemplateProperty,
} from '@/types/template';

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

export interface FlattenedContract {
  pluginId: string;
  pluginName: string;
  contract: PluginContractBindingType;
}

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
 * 从 TemplateEditor 抽出，收敛约 450 行内联渲染逻辑。
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

  const bindings = prop.contractBindings || [];
  const fieldKey = prop.id;

  // 自动推导：contractField: true 但无硬编码 bindings 时，从已安装插件 manifest 匹配
  const derivedBindings =
    bindings.length === 0 && prop.contractField && editContractTypeId
      ? deriveContractBindings(editContractTypeId, prop.id, installedPlugins)
      : [];
  const effectiveBindings = bindings.length > 0 ? bindings : derivedBindings;
  const currentContractId = selectedContractId;
  const currentRoleId = selectedRoleId;

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

      {/* 插件绑定折叠区域 */}
      <div style={{ paddingLeft: 10, marginTop: 2, marginBottom: 6 }}>
        <button
          type="button"
          onClick={() => onToggleBindingExpanded(fieldKey, idx)}
          className="interactive-ghost"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '4px 8px',
            borderRadius: 6,
            borderWidth: 1,
            borderStyle: 'solid',
            borderColor: 'transparent',
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
              transform: isExpanded ? 'rotate(90deg)' : 'rotate(0deg)',
              transition: 'transform 0.15s ease',
              display: 'inline-flex',
              flexShrink: 0,
            }}
          >
            <ChevronRight size={14} />
          </span>
          {t('settings:plugin_binding', { defaultValue: '插件绑定' })}
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
                ? t('common:collapse', { defaultValue: '收起' })
                : t('settings:click_to_configure', { defaultValue: '点击配置' })}
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
                          onClick={() => handleRemoveBinding(b.contractTypeId, b.roleId)}
                          className="interactive-fade"
                          style={{
                            background: 'none',
                            border: 'none',
                            cursor: 'pointer',
                            padding: '0 2px',
                            color: 'var(--accent-primary)',
                            fontSize: 14,
                            lineHeight: 1,
                          }}
                          title={t('common:remove', { defaultValue: '移除' })}
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
                    onSelectedContractChange(fieldKey, e.target.value);
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
                    {t('settings:select_plugin_contract', { defaultValue: '选择插件契约' })}
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
                    onSelectedRoleChange(fieldKey, e.target.value);
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
                  <option value="">{t('settings:select_role', { defaultValue: '选择角色' })}</option>
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
                  {t('common:add', { defaultValue: 'Add' })}
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
                {t('settings:no_plugin_contracts_available', { defaultValue: '暂无已安装的插件契约（需安装含有角色定义的插件）' })}
              </div>
            )}
          </div>
        )}
      </div>
    </React.Fragment>
  );
}
