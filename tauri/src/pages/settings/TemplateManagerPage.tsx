import { useEffect, useMemo, useCallback, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Button } from '@/components/ui/Button';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { usePluginStore } from '@/stores/pluginStore';
import { LayoutTemplate, Pencil, Plus, BookOpen, Info } from 'lucide-react';
import buttonStyles from '@/components/ui/Button.module.css';
import { logger } from '@/lib/logger';
import { deriveSampleTemplateBindings } from '@/lib/sampleTemplates';
import type { SampleTemplate } from '@/lib/sampleTemplates';
import { DeleteConfirmDialog } from '@/components/template/DeleteConfirmDialog';
import { TemplateDetailModal } from '@/components/template/TemplateDetailModal';
import { retentionPeriodDays } from '@/stores/trashStore';
import { ICON_SIZE } from '@/lib/constants';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import type { PluginManifest } from '@/lib/plugin';
import { useTemplateEditor } from '@/hooks/useTemplateEditor';
import { TemplateListSection, type ListTemplate } from './TemplateListSection';
import { TemplateEditorModal } from './TemplateEditorModal';
import { SampleGallerySection } from './SampleGallerySection';

const EMPTY_PLUGINS: PluginManifest[] = [];

const SYSTEM_PAGES = ['identity', 'travel', 'financial', 'professional'] as const;

/**
 * 模板管理页（P224-③ 拆分后为编排层）：
 * 列表/详情/删除/示例画廊状态与数据派生留本页，编辑器状态与操作收敛于 useTemplateEditor，
 * 列表/编辑器/画廊三面板经 props 透传。
 */
export function TemplateManagerPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['common', 'settings', 'editor']);
  // P055: 分字段 selector，避免 store 任何变化触发整页重渲染（函数引用稳定）
  const templates = useTemplateStore((s) => s.templates);
  const isLoading = useTemplateStore((s) => s.isLoading);
  const error = useTemplateStore((s) => s.error);
  const loadTemplates = useTemplateStore((s) => s.loadTemplates);
  const deleteTemplate = useTemplateStore((s) => s.deleteTemplate);
  const createTemplate = useTemplateStore((s) => s.createTemplate);
  const settings = useSettingsStore((s) => s.settings);
  const loadCustomPages = useSettingsStore((s) => s.loadCustomPages);
  const trashRetention = settings.trashRetention;
  const accountId = useAuthStore((s) => s.currentAccount?.id) || '';
  const showToast = useUiStore((s) => s.showToast);
  const installedPlugins = usePluginStore((s) => s.installedPlugins) ?? EMPTY_PLUGINS;
  const loadInstalled = usePluginStore((s) => s.loadInstalled);
  const editor = useTemplateEditor();

  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [detailTemplate, setDetailTemplate] = useState<ListTemplate | null>(null);
  const [showSampleGallery, setShowSampleGallery] = useState(false);
  const [pageFilter, setPageFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    // 避免切换页面时重复触发 loading 导致闪烁；只在首次无数据时加载
    if (templates.length === 0) {
      loadTemplates().catch((err) => logger.warn('[TemplateManager] Load templates failed:', err));
    }
    if (accountId)
      loadCustomPages(accountId).catch((err) =>
        logger.warn('[TemplateManager] Load custom pages failed:', err),
      );
  }, [loadTemplates, accountId, loadCustomPages, templates.length]);

  // 独立的 useEffect 加载插件列表（与模板/页面加载无关，避免 installedPlugins 变化触发不必要的重载）
  useEffect(() => {
    if (installedPlugins.length === 0) {
      // P042: 插件列表加载失败不再静默吞错（降级表现为插件市场数据缺失，需可诊断）。
      loadInstalled().catch((err) =>
        logger.warn('[TemplateManager] Load installed plugins failed:', err),
      );
    }
  }, [installedPlugins.length, loadInstalled]);

  const allTemplates: ListTemplate[] = useMemo(() => {
    return templates.map((ut) => ({
      id: ut.id,
      name: ut.name,
      category: ut.category || 'identity',
      contractTypeId: ut.contractTypeId,
      properties: ut.properties.map((p) => ({
        id: p.id,
        name: p.name,
        type: p.type,
        sensitivityLevel: p.sensitivityLevel || 'internal',
        deprecatedAt: p.deprecatedAt,
        contractField: p.contractField,
        contractBindings: p.contractBindings,
      })),
    }));
  }, [templates]);

  const resolvePageLabel = useCallback(
    (category: string): { name: string; deleted: boolean } => {
      if (SYSTEM_PAGES.includes(category as (typeof SYSTEM_PAGES)[number])) {
        return { name: t(`navigation:${category}`), deleted: false };
      }
      const cp = settings.customPages.find((p) => p.id === category);
      if (cp) {
        return { name: cp.name, deleted: !!cp.deletedAt };
      }
      return { name: t('settings:deleted_page', { defaultValue: '（页面已删除）' }), deleted: true };
    },
    [settings.customPages, t],
  );

  const pageOptions = useMemo(
    () => [
      { id: 'all', label: t('settings:filter_all') },
      ...SYSTEM_PAGES.map((id) => ({ id, label: t(`navigation:${id}`) })),
      ...(settings.customPages || [])
        .filter((p) => !p.deletedAt)
        .map((p) => ({ id: p.id, label: p.name })),
    ],
    [t, settings.customPages],
  );

  const filteredTemplates = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return allTemplates.filter((tpl) => {
      const matchesPage = pageFilter === 'all' || tpl.category === pageFilter;
      const matchesSearch = !q || tpl.name.toLowerCase().includes(q);
      return matchesPage && matchesSearch;
    });
  }, [allTemplates, pageFilter, searchQuery]);

  const handleDelete = (id: string, name: string) => {
    setConfirmDelete({ id, name });
  };

  const doDelete = async () => {
    if (!confirmDelete) return;
    try {
      await deleteTemplate(confirmDelete.id);
      setConfirmDelete(null);
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:delete_failed')}: ${e}` });
    }
  };

  const handleUseSample = useCallback(
    async (sample: SampleTemplate) => {
      try {
        // 从示例模板创建时，使用共享推导函数补齐 contractBindings
        const derivedProperties = deriveSampleTemplateBindings(sample, installedPlugins).map(
          (p) => ({
            id: p.id,
            name: p.name,
            type: p.type,
            sensitivityLevel: p.sensitivityLevel,
            options: p.options,
            contractField: p.contractField,
            contractBindings: p.contractBindings,
          }),
        );
        await createTemplate(
          sample.name,
          sample.icon,
          sample.category,
          derivedProperties,
          sample.contractTypeId,
        );
        setShowSampleGallery(false);
      } catch (e) {
        showToast({ type: 'error', message: `${t('common:save_failed')}: ${e}` });
        throw e;
      }
    },
    [installedPlugins, createTemplate, showToast, t],
  );

  const templateGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_template_title', { defaultValue: 'Template Guide' }),
        steps: [
          {
            icon: LayoutTemplate,
            title: t('common:guide_template_step1_title', { defaultValue: 'View Templates' }),
            description:
              t('common:guide_template_step1_desc', { defaultValue: 'Browse templates by page category. Use the search and page filters to find the template you need.' }),
          },
          {
            icon: Pencil,
            title: t('common:guide_template_step2_title', { defaultValue: 'Create & Edit' }),
            description:
              t('common:guide_template_step2_desc', { defaultValue: 'Create a new template or edit an existing one. Define fields, types, and sensitivity levels.' }),
          },
          {
            icon: BookOpen,
            title: t('common:guide_template_step3_title', { defaultValue: 'Sample Templates' }),
            description:
              t('common:guide_template_step3_desc', { defaultValue: 'Use the sample template gallery to quickly add commonly used templates to your vault.' }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_templates', { defaultValue: 'Template Management' }),
            description:
              t('common:guide_help_templates_desc', { defaultValue: 'Create, edit, and manage object templates' }),
            href: '/help?id=templates',
          },
        ],
      },
    ],
    [t],
  );

  const from = (location.state as { from?: string } | null)?.from;
  const handleBack = () => {
    if (from && from.startsWith('/editor')) {
      navigate(-1);
    } else if (from && from.startsWith('/')) {
      navigate(from);
    } else {
      navigate('/settings');
    }
  };

  return (
    <AppShell
      title={t('settings:template_manager_title', { defaultValue: '模板管理' })}
      onBack={handleBack}
      actions={
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <PageGuideButton pages={templateGuidePages} />
          <Button
            variant="secondary"
            className={`${buttonStyles.hideLabelOnMobile} ${buttonStyles.compactMobile}`}
            aria-label={t('settings:sample_templates', { defaultValue: 'Sample templates' })}
            onClick={() => setShowSampleGallery(true)}
          >
            <BookOpen size={ICON_SIZE.md} style={{ marginRight: 4 }} />
            <span className={buttonStyles.label}>
              {t('settings:sample_templates', { defaultValue: '模板示例' })}
            </span>
          </Button>
          <Button
            variant="secondary"
            className={`${buttonStyles.hideLabelOnMobile} ${buttonStyles.compactMobile}`}
            aria-label={t('settings:new_template', { defaultValue: 'New template' })}
            onClick={editor.openCreate}
          >
            <Plus size={ICON_SIZE.md} style={{ marginRight: 4 }} />
            <span className={buttonStyles.label}>{t('settings:new_template', { defaultValue: '新建模板' })}</span>
          </Button>
        </div>
      }
    >
      <PageContainer variant="medium" gap="default">
        <TemplateListSection
          isLoading={isLoading}
          error={error}
          allTemplates={allTemplates}
          filteredTemplates={filteredTemplates}
          templates={templates}
          pageFilter={pageFilter}
          onPageFilterChange={(v) => {
            if (v) setPageFilter(v);
          }}
          pageOptions={pageOptions}
          searchQuery={searchQuery}
          onSearchQueryChange={setSearchQuery}
          resolvePageLabel={resolvePageLabel}
          onOpenEdit={editor.openEdit}
          onOpenDetail={setDetailTemplate}
          onDelete={handleDelete}
        />
      </PageContainer>

      {/* Edit / Create Dialog */}
      <TemplateEditorModal editor={editor} />

      {/* Template detail modal */}
      <TemplateDetailModal
        detailTemplate={detailTemplate}
        templates={templates}
        pageLabel={resolvePageLabel}
        onClose={() => setDetailTemplate(null)}
        onEdit={(id) => {
          const ut = templates.find((u) => u.id === id);
          if (ut) editor.openEdit(ut);
        }}
      />

      {/* Sample templates gallery + detail */}
      <SampleGallerySection
        isOpen={showSampleGallery}
        onClose={() => setShowSampleGallery(false)}
        onUseSample={handleUseSample}
      />

      {editor.confirmDialog}

      {/* Delete confirmation dialog */}
      {confirmDelete &&
        (() => {
          const retentionDays = retentionPeriodDays(trashRetention);
          const bodyKey =
            retentionDays > 0
              ? 'settings:template_delete_confirm_body'
              : 'settings:template_delete_confirm_body_never';
          return (
            <DeleteConfirmDialog
              name={confirmDelete.name}
              title={t('settings:template_delete_confirm_title')}
              body={t(bodyKey, {
                name: confirmDelete.name,
                days: retentionDays > 0 ? String(retentionDays) : '',
              })}
              onCancel={() => setConfirmDelete(null)}
              onConfirm={doDelete}
            />
          );
        })()}
    </AppShell>
  );
}
