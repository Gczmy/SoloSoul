import { useTranslation } from 'react-i18next';
import type { ContractRoleBinding } from '@/types/template';
import type { FlattenedContract } from './TemplateFieldBindingSection';

export interface TemplateFieldBindingListProps {
  bindings: ContractRoleBinding[];
  isDerived: boolean;
  flattenContracts: FlattenedContract[];
  onRemoveBinding: (contractTypeId: string, roleId: string) => void;
}

/**
 * 已绑定契约/角色标签列表（W001 拆分：展示子组件）。
 * 从 TemplateFieldBindingSection 抽出——收敛已绑定标签渲染与契约/角色信息查找。
 */
export function TemplateFieldBindingList({
  bindings,
  isDerived,
  flattenContracts,
  onRemoveBinding,
}: TemplateFieldBindingListProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);

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
    <div
      style={{
        display: 'flex',
        flexWrap: 'wrap',
        gap: 6,
        marginBottom: 4,
      }}
    >
      {bindings.map((b) => {
        const ci = getContractInfo(b.contractTypeId);
        const ri = getRoleInfo(b.contractTypeId, b.roleId);
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
                onClick={() => onRemoveBinding(b.contractTypeId, b.roleId)}
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
  );
}
