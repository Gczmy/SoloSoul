import { useTranslation } from 'react-i18next';
import type { PluginContractRole } from '@/lib/plugin';
import type { FlattenedContract } from './TemplateFieldBindingSection';

export interface TemplateFieldBindingFormProps {
  flattenContracts: FlattenedContract[];
  selectedContractId: string;
  selectedRoleId: string;
  availableRoles: PluginContractRole[];
  fieldKey: string;
  onSelectedContractChange: (fieldKey: string, value: string) => void;
  onSelectedRoleChange: (fieldKey: string, value: string) => void;
  onAddBinding: () => void;
}

/**
 * 添加绑定表单（W001 拆分：展示子组件）。
 * 从 TemplateFieldBindingSection 抽出——收敛契约/角色选择与添加按钮；
 * 无可用契约时展示空态提示。
 */
export function TemplateFieldBindingForm({
  flattenContracts,
  selectedContractId,
  selectedRoleId,
  availableRoles,
  fieldKey,
  onSelectedContractChange,
  onSelectedRoleChange,
  onAddBinding,
}: TemplateFieldBindingFormProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);

  const currentContractId = selectedContractId;
  const currentRoleId = selectedRoleId;

  return (
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
        onClick={onAddBinding}
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
          cursor: !currentContractId || !currentRoleId ? 'not-allowed' : 'pointer',
          whiteSpace: 'nowrap',
          transition: 'background 0.15s',
        }}
      >
        {t('common:add', { defaultValue: 'Add' })}
      </button>
    </div>
  );
}
