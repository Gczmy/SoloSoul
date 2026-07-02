import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Pencil, LayoutTemplate } from 'lucide-react';
import { motion } from 'framer-motion';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SensitivityBadges } from './SensitivityBadges';
import { PluginBadge } from './PluginBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import type {
  PropertyType,
  SensitivityLevel,
  UserTemplate,
  ContractRoleBinding,
} from '@/types/template';
import { ICON_SIZE } from '@/lib/iconSizes';
import { usePluginStore } from '@/stores/pluginStore';
import { deriveContractBindings } from '@/lib/plugin';

interface DetailProperty {
  id: string;
  name: string;
  type: string;
  sensitivityLevel?: string;
  deprecatedAt?: string;
  contractField?: boolean;
  contractBindings?: ContractRoleBinding[];
}

interface ListTemplate {
  id: string;
  name: string;
  category: string;
  contractTypeId?: string;
  properties: DetailProperty[];
}

interface TemplateDetailModalProps {
  detailTemplate: ListTemplate | null;
  templates: UserTemplate[];
  pageLabel: (category: string) => { name: string; deleted: boolean };
  onClose: () => void;
  onEdit: (id: string) => void;
}

export function TemplateDetailModal({
  detailTemplate,
  templates,
  pageLabel,
  onClose,
  onEdit,
}: TemplateDetailModalProps) {
  const { t } = useTranslation(['common', 'settings']);
  const installedPlugins = usePluginStore((s) => s.installedPlugins);
  const loadInstalled = usePluginStore((s) => s.loadInstalled);

  // 确保插件列表已加载（用于 contractField 推导）
  useEffect(() => {
    if (installedPlugins.length === 0) {
      loadInstalled().catch(() => {});
    }
  }, [installedPlugins.length, loadInstalled]);

  if (!detailTemplate) return null;

  const page = pageLabel(detailTemplate.category || 'identity');

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)',
        backdropFilter: 'blur(4px)',
      }}
      onClick={onClose}
    >
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.2 }}
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: '28px 32px',
          maxWidth: 520,
          width: '90%',
          maxHeight: '80vh',
          overflowY: 'auto',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        {/* Title row */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 20,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <LayoutTemplate size={ICON_SIZE['2xl']} color="var(--accent-primary)" />
            <div>
              <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 700, margin: 0 }}>
                {detailTemplate.name}
              </h2>
              <span
                style={{
                  fontSize: 'var(--text-badge)',
                  color: 'var(--text-tertiary)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                }}
              >
                <PluginBadge
                  contractTypeId={detailTemplate.contractTypeId}
                  size="sm"
                />
                <span
                  style={
                    page.deleted ? { textDecoration: 'line-through', opacity: 0.6 } : undefined
                  }
                >
                  {page.name}
                </span>
                <span>·</span>
                <span>
                  {detailTemplate.properties.length} {t('settings:template_fields') || '个字段'}
                </span>
                <SensitivityBadges properties={detailTemplate.properties} />
              </span>
            </div>
          </div>
          <button
            onClick={onClose}
            style={{
              padding: 6,
              borderRadius: 8,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
            }}
          >
            <X size={ICON_SIZE.xl} />
          </button>
        </div>

        {/* Divider */}
        <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 20 }} />

        {/* Fields */}
        {detailTemplate.properties.length === 0 ? (
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
              padding: '16px 0',
            }}
          >
            {t('settings:empty_template_hint') || '此模板暂无字段'}
          </p>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {detailTemplate.properties.map((prop) => (
              <div
                key={prop.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 12,
                  padding: '10px 14px',
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  border: '1px solid var(--border-subtle)',
                  opacity: prop.deprecatedAt ? 0.7 : 1,
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    flex: 1,
                    minWidth: 0,
                  }}
                >
                  <span
                    style={{
                      color: 'var(--text-tertiary)',
                      display: 'flex',
                      alignItems: 'center',
                    }}
                  >
                    <FieldTypeIcon type={prop.type as PropertyType} size={ICON_SIZE.sm} />
                  </span>
                  <span
                    style={{
                      fontSize: 'var(--text-body)',
                      fontWeight: 500,
                      color: 'var(--text-primary)',
                      textDecoration: prop.deprecatedAt ? 'line-through' : 'none',
                    }}
                  >
                    {prop.name}
                  </span>
                  {(() => {
                    const resolvedBindings = prop.contractBindings && prop.contractBindings.length > 0
                      ? prop.contractBindings
                      : (prop.contractField && detailTemplate.contractTypeId
                          ? deriveContractBindings(
                              detailTemplate.contractTypeId,
                              prop.id,
                              installedPlugins,
                            )
                          : []);
                    if (resolvedBindings.length > 0) {
                      return (
                        <PluginBadge
                          contractTypeId={resolvedBindings[0].contractTypeId}
                          size="sm"
                          variant="icon"
                        />
                      );
                    }
                    // 插件未安装但字段标记为 contractField ——显示灰色占位
                    if (prop.contractField) {
                      return (
                        <span
                          style={{
                            fontSize: 'var(--text-badge)',
                            padding: '1px 4px',
                            borderRadius: 3,
                            background: 'var(--accent-primary-soft, rgba(99,102,241,0.12))',
                            color: 'var(--accent-primary, #6366f1)',
                            opacity: 0.6,
                          }}
                        >
                          {t('settings:plugin_badge_label', '插件')}
                        </span>
                      );
                    }
                    return null;
                  })()}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <SensitivityBadge
                    level={(prop.sensitivityLevel || 'internal') as SensitivityLevel}
                  />
                  {prop.deprecatedAt && <DeprecatedBadge />}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Actions */}
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 24 }}>
          <Button variant="secondary" onClick={onClose}>
            {t('common:close') || '关闭'}
          </Button>
          <Button
            variant="secondary"
            style={{ border: '1px solid var(--accent-primary)', color: 'var(--accent-primary)' }}
            onClick={() => {
              const ut = templates.find((u) => u.id === detailTemplate.id);
              if (ut) {
                onClose();
                onEdit(detailTemplate.id);
              }
            }}
          >
            <Pencil size={ICON_SIZE.md} style={{ marginRight: 4 }} />
            {t('common:edit') || '编辑'}
          </Button>
        </div>
      </motion.div>
    </div>
  );
}
