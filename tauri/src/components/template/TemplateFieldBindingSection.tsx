import { useTranslation } from 'react-i18next';
import { ChevronRight } from 'lucide-react';
import {
  deriveContractBindings,
  type PluginContractBinding as PluginContractBindingType,
  type PluginManifest,
} from '@/lib/plugin';
import type {
  ContractRoleBinding,
  TemplateProperty,
} from '@/types/template';
import { TemplateFieldBindingList } from './TemplateFieldBindingList';
import { TemplateFieldBindingForm } from './TemplateFieldBindingForm';

export interface FlattenedContract {
  pluginId: string;
  pluginName: string;
  contract: PluginContractBindingType;
}

export interface TemplateFieldBindingSectionProps {
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
  onUpdatePropertyContractBindings: (index: number, bindings: ContractRoleBinding[]) => void;
  onContractTypeIdChange: (v: string) => void;
}

/**
 * 模板字段的插件契约绑定折叠区（P046 拆分：展示子组件）。
 * 从 TemplateFieldRow 抽出——收敛绑定折叠区编排：自动推导 bindings、
 * 折叠头/展开态、已绑定列表与添加绑定表单的组合（W001 再拆后本组件为纯组合层）。
 */
export function TemplateFieldBindingSection({
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
  onUpdatePropertyContractBindings,
  onContractTypeIdChange,
}: TemplateFieldBindingSectionProps) {
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

  return (
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
            <TemplateFieldBindingList
              bindings={effectiveBindings}
              isDerived={bindings.length === 0}
              flattenContracts={flattenContracts}
              onRemoveBinding={handleRemoveBinding}
            />
          )}

          {/* 添加绑定 */}
          {flattenContracts.length > 0 ? (
            <TemplateFieldBindingForm
              flattenContracts={flattenContracts}
              selectedContractId={selectedContractId}
              selectedRoleId={selectedRoleId}
              availableRoles={availableRoles}
              fieldKey={fieldKey}
              onSelectedContractChange={onSelectedContractChange}
              onSelectedRoleChange={onSelectedRoleChange}
              onAddBinding={handleAddBinding}
            />
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
  );
}
