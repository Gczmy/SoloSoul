import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { PluginBadge } from './PluginBadge';
import { ICON_SIZE } from '@/lib/constants';
import type { PropertyType } from '@/types/template';

interface TemplateFieldRowItemProps {
  type: PropertyType;
  name: string;
  /** 字段已弃用：行内降低透明度 + 名称删除线 */
  deprecated?: boolean;
  /** 契约绑定字段：名称后显示契约徽标 */
  contractField?: boolean;
  contractTypeId?: string;
  /** 右侧展示区（敏感级别徽标 / 类型标签 / 弃用徽标等） */
  right?: ReactNode;
}

/**
 * P226: 模板详情字段行骨架——图标 + 名称 + 契约徽标 + 右侧插槽。
 *
 * 收敛自 TemplateDetailModal 与 SampleTemplateDetail 两处逐字节相同的字段行容器：
 * 行背景/内边距/圆角/图标/名称（含 __dynamic_group__ 翻译）/契约徽标 完全一致，
 * 仅右侧展示内容（SensitivityBadge / DeprecatedBadge / 类型标签）与弃用态不同，以 props 参数化。
 */
export function TemplateFieldRowItem({
  type,
  name,
  deprecated = false,
  contractField = false,
  contractTypeId,
  right,
}: TemplateFieldRowItemProps) {
  const { t } = useTranslation(['editor']);

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 12,
        padding: '10px 14px',
        borderRadius: 8,
        background: 'var(--bg-toolbar)',
        border: '1px solid var(--border-subtle)',
        opacity: deprecated ? 0.7 : 1,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
        <span style={{ color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center' }}>
          <FieldTypeIcon type={type} size={ICON_SIZE.sm} />
        </span>
        <span
          style={{
            fontSize: 'var(--text-body)',
            fontWeight: 500,
            color: 'var(--text-primary)',
            textDecoration: deprecated ? 'line-through' : 'none',
          }}
        >
          {name === '__dynamic_group__' ? t('editor:field_types.dynamic_group') : name}
        </span>
        {contractField && contractTypeId ? (
          <PluginBadge contractTypeId={contractTypeId} size="sm" variant="icon" />
        ) : null}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>{right}</div>
    </div>
  );
}
