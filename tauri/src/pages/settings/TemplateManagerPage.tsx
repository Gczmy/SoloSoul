import { useState, useEffect, useMemo, useCallback } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Dialog } from '@/components/ui/Dialog';
import { Input } from '@/components/ui/Input';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import {
  LayoutTemplate,
  Trash2,
  Pencil,
  Plus,
  BookOpen,
} from 'lucide-react';
import type {
  UserTemplate,
  TemplateProperty,
  PropertyType,
} from '@/types/template';
import { resolveCustomIcon } from '@/lib/pageIcons';
import { SampleTemplateGallery } from '@/components/template/SampleTemplateGallery';
import { SampleTemplateDetail } from '@/components/template/SampleTemplateDetail';
import type { SampleTemplate } from '@/lib/sampleTemplates';
import { TemplateEditor } from '@/components/template/TemplateEditor';
import { TemplateDetailModal } from '@/components/template/TemplateDetailModal';
import { DeleteConfirmDialog } from '@/components/template/DeleteConfirmDialog';
import { SensitivityBadges } from '@/components/template/SensitivityBadges';
import { PluginBadge } from '@/components/template/PluginBadge';
import { retentionPeriodDays } from '@/stores/trashStore';
import { ICON_SIZE } from '@/lib/iconSizes';


const SYSTEM_PAGES = ['identity', 'travel', 'financial', 'professional'] as const;

interface ListTemplate {
  id: string;
  name: string;
  category: string;
  properties: Array<{
    id: string;
    name: string;
    type: string;
    sensitivityLevel?: string;
    deprecatedAt?: string;
  }>;
}

export function TemplateManagerPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['common', 'settings', 'editor']);
  const {
    templates,
    isLoading,
    error,
    loadTemplates,
    deleteTemplate,
    updateTemplate,
    createTemplate,
    checkFieldUsage,
  } = useTemplateStore();
  const { settings, loadCustomPages } = useSettingsStore();
  const trashRetention = settings.trashRetention;
  const accountId = useAuthStore((s) => s.currentAccount?.id) || '';
  const showToast = useUiStore((s) => s.showToast);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  const [editingTemplate, setEditingTemplate] = useState<UserTemplate | null>(null);
  const [isNewTemplate, setIsNewTemplate] = useState(false);
  const [editName, setEditName] = useState('');
  const [editCategory, setEditCategory] = useState<string>('identity');
  const [editIconId, setEditIconId] = useState<string>('document');
  const [editProperties, setEditProperties] = useState<TemplateProperty[]>([]);

  const [newFieldType, setNewFieldType] = useState<PropertyType>('text');
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [detailTemplate, setDetailTemplate] = useState<ListTemplate | null>(null);
  const [showDeprecated, setShowDeprecated] = useState(false);
  const [fieldUsageMap, setFieldUsageMap] = useState<
    Record<string, { active: number; softDeleted: number }>
  >({});
  const [showSampleGallery, setShowSampleGallery] = useState(false);
  const [selectedSample, setSelectedSample] = useState<SampleTemplate | null>(null);
  const [pageFilter, setPageFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    loadTemplates().catch(() => {});
    if (accountId) loadCustomPages(accountId).catch(() => {});
  }, [loadTemplates, accountId, loadCustomPages]);



  // Load field usage for deprecated fields
  const loadFieldUsage = useCallback(async () => {
    if (!editingTemplate) {
      setFieldUsageMap({});
      return;
    }
    const deprecated = editProperties.filter((p) => p.deprecatedAt);
    if (deprecated.length === 0) return;
    const map: Record<string, { active: number; softDeleted: number }> = {};
    await Promise.all(
      deprecated.map(async (p) => {
        try {
          const usage = await checkFieldUsage(editingTemplate.id, p.id);
          map[p.id] = usage;
        } catch {
          /* ignore */
        }
      }),
    );
    setFieldUsageMap(map);
  }, [editingTemplate, editProperties, checkFieldUsage]);

  useEffect(() => {
    loadFieldUsage();
  }, [loadFieldUsage]);

  const allTemplates: ListTemplate[] = useMemo(() => {
    return templates.map((ut) => ({
      id: ut.id,
      name: ut.name,
      category: ut.category || 'identity',
      properties: ut.properties.map((p) => ({
        id: p.id,
        name: p.name,
        type: p.type,
        sensitivityLevel: p.sensitivityLevel || 'internal',
        deprecatedAt: p.deprecatedAt,
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
      return { name: t('settings:deleted_page') || '（页面已删除）', deleted: true };
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

  const openEdit = (tpl: UserTemplate) => {
    setIsNewTemplate(false);
    setEditingTemplate(tpl);
    setEditName(tpl.name);
    setEditCategory(tpl.category || 'identity');
    setEditIconId(tpl.iconId || 'document');
    setEditProperties([...tpl.properties]);
  };

  const openCreate = () => {
    setIsNewTemplate(true);
    setEditingTemplate({
      id: '',
      accountId: '',
      name: '',
      category: 'identity',
      properties: [],
      createdAt: '',
    } as UserTemplate);
    setEditName('');
    setEditCategory('identity');
    setEditIconId('document');
    setEditProperties([]);
  };

  const closeEdit = () => {
    setIsNewTemplate(false);
    setEditingTemplate(null);
    setEditName('');
    setEditCategory('identity');
    setEditIconId('document');
    setEditProperties([]);
  };

  const saveEdit = async () => {
    const name = editName.trim();
    if (!name) {
      showToast({ type: 'warning', message: t('common:name_required') || '请输入模板名称' });
      return;
    }
    try {
      if (isNewTemplate) {
        await createTemplate(name, editIconId, editCategory, editProperties);
        await loadTemplates();
        closeEdit();
      } else if (editingTemplate) {
        await updateTemplate(editingTemplate.id, {
          name: name || editingTemplate.name,
          iconId: editIconId,
          category: editCategory,
          properties: editProperties,
        });
        closeEdit();
      }
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:save_failed')}: ${e}` });
    }
  };

  const updatePropertyName = (index: number, newName: string) => {
    setEditProperties((prev) => prev.map((p, i) => (i === index ? { ...p, name: newName } : p)));
  };

  const updatePropertyType = (index: number, newType: PropertyType) => {
    setEditProperties((prev) => prev.map((p, i) => (i === index ? { ...p, type: newType } : p)));
  };

  const updatePropertySensitivity = (index: number, level: 'public' | 'internal' | 'sensitive' | 'critical') => {
    setEditProperties((prev) =>
      prev.map((p, i) => (i === index ? { ...p, sensitivityLevel: level } : p)),
    );
  };

  const updatePropertyOptions = (index: number, options: string[]) => {
    setEditProperties((prev) =>
      prev.map((p, i) =>
        i === index ? { ...p, options: options.length > 0 ? options : undefined } : p,
      ),
    );
  };

  const removeProperty = async (index: number) => {
    const prop = editProperties[index];
    if (!prop || !editingTemplate) return;
    if (isNewTemplate) {
      setEditProperties((prev) => prev.filter((_, i) => i !== index));
      return;
    }
    try {
      const usage = await checkFieldUsage(editingTemplate.id, prop.id);
      if (usage.active > 0 || usage.softDeleted > 0) {
        requestConfirm(
          t('settings:confirm_deprecate_title'),
          t('settings:confirm_deprecate_body', {
            activeCount: usage.active,
            softDeletedCount: usage.softDeleted,
          }),
          () => {
            setEditProperties((prev) =>
              prev.map((p, i) =>
                i === index ? { ...p, deprecatedAt: new Date().toISOString() } : p,
              ),
            );
            setFieldUsageMap((prev) => ({ ...prev, [prop.id]: usage }));
          },
          { confirmLabel: t('common:confirm'), cancelLabel: t('common:cancel') },
        );
      } else {
        setEditProperties((prev) => prev.filter((_, i) => i !== index));
      }
    } catch {
      setEditProperties((prev) =>
        prev.map((p, i) => (i === index ? { ...p, deprecatedAt: new Date().toISOString() } : p)),
      );
    }
  };

  const restoreProperty = (index: number) => {
    setEditProperties((prev) =>
      prev.map((p, i) => (i === index ? { ...p, deprecatedAt: undefined } : p)),
    );
  };

  const permanentlyRemoveProperty = (index: number) => {
    setEditProperties((prev) => prev.filter((_, i) => i !== index));
  };

  const addProperty = () => {
    const newProp: TemplateProperty = {
      id: crypto.randomUUID(),
      name: t('settings:new_field_name') || '新字段',
      type: newFieldType,
      sensitivityLevel: 'internal',
    };
    setEditProperties((prev) => [...prev, newProp]);
  };

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
      title={t('settings:template_manager_title') || '模板管理'}
      onBack={handleBack}
      actions={
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Button variant="secondary" onClick={() => setShowSampleGallery(true)}>
            <BookOpen size={ICON_SIZE.md} style={{ marginRight: 4 }} />
            {t('settings:sample_templates') || '模板示例'}
          </Button>
          <Button variant="secondary" style={{ border: '1px solid var(--accent-primary)', color: 'var(--accent-primary)' }} onClick={openCreate}>
            <Plus size={ICON_SIZE.md} style={{ marginRight: 4 }} />
            {t('settings:new_template') || '新建模板'}
          </Button>
        </div>
      }
    >
      <PageContainer variant="medium" gap="default">
        {isLoading && <LoadingPlaceholder variant="base" minHeight={120} />}
        {error && <div style={{ color: 'var(--error)' }}>{error}</div>}

        {!isLoading && !error && allTemplates.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <Input
              placeholder={t('settings:search_templates') || '搜索模板...'}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onClear={() => setSearchQuery('')}
            />
            <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
              {pageOptions.map((opt) => {
                const isActive = pageFilter === opt.id;
                return (
                  <button
                    key={opt.id}
                    type="button"
                    onClick={() => setPageFilter(opt.id)}
                    aria-pressed={isActive}
                    onMouseEnter={!isActive ? (e) => {
                      e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                      e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    } : undefined}
                    onMouseLeave={!isActive ? (e) => {
                      e.currentTarget.style.background = 'var(--bg-toolbar)';
                      e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    } : undefined}
                    style={{
                      padding: '5px 12px',
                      borderRadius: 6,
                      border: isActive ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
                      background: isActive ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'var(--bg-toolbar)',
                      color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                      boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                      fontSize: 'var(--text-sm)',
                      cursor: 'pointer',
                      transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                    }}
                  >
                    {opt.label}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {!isLoading && allTemplates.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
            <LayoutTemplate size={ICON_SIZE['5xl']} style={{ marginBottom: 12, opacity: 0.4 }} />
            <div>{t('settings:no_templates') || '暂无模板'}</div>
            <div style={{ fontSize: 'var(--text-caption)', marginTop: 4 }}>
              {t('settings:no_templates_hint') || '点击右上角"新建模板"创建'}
            </div>
          </div>
        )}

        {!isLoading && allTemplates.length > 0 && filteredTemplates.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
            <div>{t('settings:no_templates_filtered') || '没有符合筛选条件的模板'}</div>
          </div>
        )}

        {filteredTemplates.map((tpl) => {
          const ut = templates.find((u) => u.id === tpl.id);
          const TemplateIcon = ut?.iconId ? resolveCustomIcon(ut.iconId) : LayoutTemplate;
          return (
          <Card key={tpl.id} interactive onClick={() => setDetailTemplate(tpl)}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <TemplateIcon size={ICON_SIZE.xl} />
                <div>
                  <div
                    style={{
                      fontSize: 'var(--text-sm)',
                      fontWeight: 500,
                      display: 'flex',
                      alignItems: 'center',
                      gap: 6,
                    }}
                  >
                    {tpl.name}
                    <PluginBadge
                      contractTypeId={templates.find((u) => u.id === tpl.id)?.contractTypeId}
                      size="sm"
                    />
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-badge)',
                      color: 'var(--text-tertiary)',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      flexWrap: 'wrap',
                    }}
                  >
                    {(() => {
                      const page = resolvePageLabel(tpl.category);
                      return (
                        <span
                          style={
                            page.deleted
                              ? { textDecoration: 'line-through', opacity: 0.6 }
                              : undefined
                          }
                        >
                          {page.name}
                        </span>
                      );
                    })()}
                    <span>·</span>
                    <span>
                      {tpl.properties.length} {t('settings:template_fields') || '个字段'}
                    </span>
                    <SensitivityBadges properties={tpl.properties} />
                  </div>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 8 }} onClick={(e) => e.stopPropagation()}>
                <Button
                  variant="tertiary"
                  size="sm"
                  onClick={() => {
                    const ut = templates.find((u) => u.id === tpl.id);
                    if (ut) openEdit(ut);
                  }}
                >
                  <Pencil size={ICON_SIZE.md} />
                </Button>
                <Button
                  variant="tertiary"
                  size="sm"
                  onClick={() => handleDelete(tpl.id, tpl.name)}
                  style={{ color: '#e74c3c' }}
                >
                  <Trash2 size={ICON_SIZE.md} />
                </Button>
              </div>
            </div>
          </Card>
        );}
        )}
      </PageContainer>

      {/* Edit / Create Dialog */}
      <Dialog
        isOpen={!!editingTemplate}
        onClose={closeEdit}
        title={
          isNewTemplate
            ? t('settings:new_template') || '新建模板'
            : t('settings:edit_template') || '编辑模板'
        }
      >
        <TemplateEditor
          editingTemplate={editingTemplate}

          editName={editName}
          editCategory={editCategory}
          editIconId={editIconId}
          editProperties={editProperties}
          newFieldType={newFieldType}
          showDeprecated={showDeprecated}
          fieldUsageMap={fieldUsageMap}
          onEditNameChange={setEditName}
          onEditCategoryChange={setEditCategory}
          onEditIconIdChange={setEditIconId}
          onNewFieldTypeChange={setNewFieldType}
          onAddProperty={addProperty}
          onUpdatePropertyName={updatePropertyName}
          onUpdatePropertyType={updatePropertyType}
          onUpdatePropertySensitivity={updatePropertySensitivity}
          onUpdatePropertyOptions={updatePropertyOptions}
          onRemoveProperty={removeProperty}
          onRestoreProperty={restoreProperty}
          onPermanentlyRemoveProperty={permanentlyRemoveProperty}
          onToggleShowDeprecated={() => setShowDeprecated((v) => !v)}
          onSave={saveEdit}
          onClose={closeEdit}
        />
      </Dialog>

      {/* Template detail modal */}
      <TemplateDetailModal
        detailTemplate={detailTemplate}
        templates={templates}
        pageLabel={resolvePageLabel}
        onClose={() => setDetailTemplate(null)}
        onEdit={(id) => {
          const ut = templates.find((u) => u.id === id);
          if (ut) openEdit(ut);
        }}
      />

      {/* Sample templates gallery */}
      <SampleTemplateGallery
        isOpen={showSampleGallery}
        onClose={() => setShowSampleGallery(false)}
        onSelect={(tpl) => {
          setSelectedSample(tpl);
        }}
      />

      {/* Sample template detail */}
      {selectedSample && (
        <SampleTemplateDetail
          template={selectedSample}
          onBack={() => setSelectedSample(null)}
          onUse={async () => {
            if (!selectedSample) return;
            try {
              await createTemplate(
                selectedSample.name,
                selectedSample.icon,
                selectedSample.category,
                selectedSample.properties.map((p) => ({
                  id: p.id,
                  name: p.name,
                  type: p.type,
                  sensitivityLevel: p.sensitivityLevel,
                  options: p.options,
                  contractField: p.contractField,
                })),
                selectedSample.contractTypeId,
              );
              setSelectedSample(null);
              setShowSampleGallery(false);
            } catch (e) {
              showToast({ type: 'error', message: `${t('common:save_failed')}: ${e}` });
            }
          }}
        />
      )}

      {confirmDialog}

      {/* Delete confirmation dialog */}
      {confirmDelete && (() => {
        const retentionDays = retentionPeriodDays(trashRetention);
        const bodyKey = retentionDays > 0
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
