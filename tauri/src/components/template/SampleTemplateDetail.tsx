import { useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { X, ArrowLeft } from 'lucide-react';
import { motion } from 'framer-motion';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { PluginBadge } from './PluginBadge';
import { TemplateFieldRowItem } from './TemplateFieldRowItem';
import type { SampleTemplate } from '@/lib/sampleTemplates';
import { deriveSampleTemplateBindings } from '@/lib/sampleTemplates';
import type { SensitivityLevel } from '@/types/template';
import { ICON_SIZE } from '@/lib/constants';
import { usePluginStore } from '@/stores/pluginStore';
import type { PluginManifest } from '@/lib/plugin';
import { logger } from '@/lib/logger';

// 模块级空数组常量，避免 ?? [] 每次创建新引用导致 ESLint useMemo 依赖 warning
const EMPTY_PLUGINS: PluginManifest[] = [];

interface SampleTemplateDetailProps {
  template: SampleTemplate;
  onBack: () => void;
  onUse: () => void;
}

export function SampleTemplateDetail({ template, onBack, onUse }: SampleTemplateDetailProps) {
  const { t } = useTranslation(['settings', 'editor', 'navigation']);
  const installedPlugins = usePluginStore((s) => s.installedPlugins) ?? EMPTY_PLUGINS;
  const loadInstalled = usePluginStore((s) => s.loadInstalled);

  // 确保插件列表已加载（用于 deriveContractBindings）
  useEffect(() => {
    if (installedPlugins.length === 0) {
      // P042: 插件列表加载失败不再静默吞错（降级表现为无契约绑定推导，需可诊断）。
      loadInstalled().catch((err) =>
        logger.warn('[SampleTemplateDetail] Load installed plugins failed:', err),
      );
    }
  }, [installedPlugins.length, loadInstalled]);

  // 保存推导结果的副本用于展示
  const derivedProperties = useMemo(
    () => deriveSampleTemplateBindings(template, installedPlugins),
    [template, installedPlugins],
  );

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal-important)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)',
        backdropFilter: 'blur(4px)',
      }}
      onClick={(e) => {
        e.stopPropagation();
        onBack();
      }}
    >
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.2 }}
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: '24px 28px',
          maxWidth: 520,
          width: '90%',
          maxHeight: '80vh',
          overflowY: 'auto',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 12,
          }}
        >
          <button
            onClick={onBack}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 10px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'transparent',
              color: 'var(--text-secondary)',
              fontSize: 'var(--text-caption)',
              cursor: 'pointer',
            }}
          >
            <ArrowLeft size={ICON_SIZE.sm} /> {t('common:back', '返回')}
          </button>
          <button
            onClick={onBack}
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

        <div style={{ display: 'flex', alignItems: 'center', gap: 8, margin: '0 0 4px' }}>
          <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 700, margin: 0 }}>
            {template.name}
          </h2>
          <PluginBadge contractTypeId={template.contractTypeId} />
        </div>
        <div
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            marginBottom: 20,
          }}
        >
          {t(`navigation:${template.category}`, template.category)} · {template.properties.length}{' '}
          {t('settings:template_fields')}
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 24 }}>
          {derivedProperties.map((prop) => (
            <TemplateFieldRowItem
              key={prop.id}
              type={prop.type}
              name={prop.name}
              contractField={prop.contractField}
              contractTypeId={template.contractTypeId}
              right={
                <>
                  <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                    {t(`editor:field_types.${prop.type}`, prop.type)}
                  </span>
                  <SensitivityBadge level={prop.sensitivityLevel as SensitivityLevel} />
                </>
              }
            />
          ))}
        </div>

        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onBack}>
            {t('common:close')}
          </Button>
          <Button
            variant="secondary"
            style={{ border: '1px solid var(--accent-primary)', color: 'var(--accent-primary)' }}
            onClick={onUse}
          >
            {t('settings:use_sample_template')}
          </Button>
        </div>
      </motion.div>
    </div>
  );
}
