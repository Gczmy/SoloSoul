import { useState, useEffect, useMemo } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Dialog } from '@/components/ui/Dialog';
import { Input } from '@/components/ui/Input';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import {
  LayoutTemplate,
  Trash2,
  Pencil,
  X,
  Save,
  Plus,
  BookOpen,
  Type,
  AlignLeft,
  Hash,
  Calendar,
  Clock,
  CheckSquare,
  List,
  ListChecks,
  Link,
  Mail,
  Phone,
  File,
} from 'lucide-react';
import { SensitivityBadge as UiSensitivityBadge } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import type {
  UserTemplate,
  TemplateProperty,
  PropertyType,
  SensitivityLevel,
} from '@/types/template';
import { SampleTemplateGallery } from '@/components/template/SampleTemplateGallery';
import { SampleTemplateDetail } from '@/components/template/SampleTemplateDetail';
import type { SampleTemplate } from '@/lib/sampleTemplates';

const PROPERTY_TYPES: PropertyType[] = [
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
];

// F023: shared field-type icon map to avoid repeating the same mapping in JSX.
const FIELD_TYPE_ICONS: Record<string, React.ReactNode> = {
  text: <Type size={14} />,
  multiline: <AlignLeft size={14} />,
  number: <Hash size={14} />,
  date: <Calendar size={14} />,
  datetime: <Clock size={14} />,
  boolean: <CheckSquare size={14} />,
  select: <List size={14} />,
  multiselect: <ListChecks size={14} />,
  url: <Link size={14} />,
  email: <Mail size={14} />,
  phone: <Phone size={14} />,
  file: <File size={14} />,
};

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

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

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

/** Overlay editor for select/multiselect field options */
function OptionsEditor({
  options,
  onChange,
  fieldName,
  fieldType,
}: {
  options: string[];
  onChange: (opts: string[]) => void;
  fieldName: string;
  fieldType: 'select' | 'multiselect';
}) {
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState('');

  const handleOpen = () => {
    setEditing(options.join('\n'));
    setOpen(true);
  };

  return (
    <>
      <button
        type="button"
        onClick={handleOpen}
        title="编辑选项"
        style={{
          height: 36,
          padding: '0 10px',
          borderRadius: 6,
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-elevated)',
          color: 'var(--text-secondary)',
          fontSize: 13,
          cursor: 'pointer',
          whiteSpace: 'nowrap',
          lineHeight: '36px',
        }}
      >
        {options.length > 0 ? `${options.length} 个选项` : '添加选项'}
      </button>
      {open && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 99999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.35)',
            backdropFilter: 'blur(4px)',
          }}
          onClick={() => setOpen(false)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 16,
              padding: '28px 32px',
              maxWidth: 420,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            <h3 style={{ margin: '0 0 4px', fontSize: 16, fontWeight: 600 }}>
              {fieldType === 'multiselect' ? '编辑多选选项' : '编辑单选选项'}
              <span
                style={{
                  fontWeight: 400,
                  color: 'var(--text-secondary)',
                  marginLeft: 8,
                  fontSize: 14,
                }}
              >
                {fieldName}
              </span>
            </h3>
            <p style={{ margin: '0 0 16px', fontSize: 12, color: 'var(--text-tertiary)' }}>
              {fieldType === 'multiselect'
                ? '每行输入一个选项，可多选'
                : '每行输入一个选项，只能选一项'}
            </p>
            <textarea
              value={editing}
              onChange={(e) => setEditing(e.target.value)}
              rows={8}
              style={{
                width: '100%',
                padding: '10px 12px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontSize: 14,
                fontFamily: 'inherit',
                resize: 'vertical',
                boxSizing: 'border-box',
                outline: 'none',
              }}
              autoFocus
            />
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 16 }}>
              <button
                type="button"
                onClick={() => setOpen(false)}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'transparent',
                  cursor: 'pointer',
                  fontSize: 14,
                  color: 'var(--text-secondary)',
                }}
              >
                取消
              </button>
              <button
                type="button"
                onClick={() => {
                  const opts = editing
                    .split('\n')
                    .map((s) => s.trim())
                    .filter(Boolean);
                  onChange(opts);
                  setOpen(false);
                }}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: 'none',
                  background: 'var(--accent-primary)',
                  cursor: 'pointer',
                  fontSize: 14,
                  color: 'white',
                }}
              >
                确定
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

function SensitivityBadges({ properties }: { properties: ListTemplate['properties'] }) {
  const present = new Set(
    properties.map((p) => (p.sensitivityLevel || 'internal') as SensitivityLevel),
  );
  const ordered = SENSITIVITY_ORDER.filter((level) => present.has(level));
  if (ordered.length === 0) return null;
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
      {ordered.map((level) => (
        <UiSensitivityBadge key={level} level={level} />
      ))}
    </div>
  );
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
  const accountId = useAuthStore((s) => s.currentAccount?.id) || '';

  const [editingTemplate, setEditingTemplate] = useState<UserTemplate | null>(null);
  const [isNewTemplate, setIsNewTemplate] = useState(false);
  const [editName, setEditName] = useState('');
  const [editCategory, setEditCategory] = useState<string>('identity');
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

  // 加载废弃字段的使用情况
  useEffect(() => {
    if (!editingTemplate) {
      setFieldUsageMap({});
      return;
    }
    const deprecated = editProperties.filter((p) => p.deprecatedAt);
    if (deprecated.length === 0) return;
    const loadAll = async () => {
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
    };
    loadAll();
  }, [editingTemplate?.id, editProperties.map((p) => p.deprecatedAt).join(',')]);

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

  const resolvePageLabel = (category: string): { name: string; deleted: boolean } => {
    if (SYSTEM_PAGES.includes(category as (typeof SYSTEM_PAGES)[number])) {
      return { name: t(`navigation:${category}`), deleted: false };
    }
    const cp = settings.customPages.find((p) => p.id === category);
    if (cp) {
      // Page exists (may be soft-deleted) → show name with strikethrough
      return { name: cp.name, deleted: !!cp.deletedAt };
    }
    // Page has been permanently purged → show generic "deleted" label
    return { name: t('settings:deleted_page') || '（页面已删除）', deleted: true };
  };

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

  const handleDelete = async (id: string, name: string) => {
    setConfirmDelete({ id, name });
  };

  const doDelete = async () => {
    if (!confirmDelete) return;
    try {
      await deleteTemplate(confirmDelete.id);
      setConfirmDelete(null);
    } catch (e) {
      alert(t('common:delete_failed') + ': ' + e);
    }
  };

  const openEdit = (tpl: UserTemplate) => {
    setIsNewTemplate(false);
    setEditingTemplate(tpl);
    setEditName(tpl.name);
    setEditCategory(tpl.category || 'identity');
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
    setEditProperties([]);
  };

  const closeEdit = () => {
    setIsNewTemplate(false);
    setEditingTemplate(null);
    setEditName('');
    setEditCategory('identity');
    setEditProperties([]);
  };

  const saveEdit = async () => {
    const name = editName.trim();
    if (!name) {
      alert(t('common:name_required') || '请输入模板名称');
      return;
    }
    try {
      if (isNewTemplate) {
        await createTemplate(name, undefined, editCategory, editProperties);
        await loadTemplates();
        closeEdit();
      } else if (editingTemplate) {
        await updateTemplate(editingTemplate.id, {
          name: name || editingTemplate.name,
          category: editCategory,
          properties: editProperties,
        });
        closeEdit();
      }
    } catch (e) {
      alert(t('common:save_failed') + ': ' + e);
    }
  };

  const updatePropertyName = (index: number, newName: string) => {
    setEditProperties((prev) => prev.map((p, i) => (i === index ? { ...p, name: newName } : p)));
  };

  const updatePropertyType = (index: number, newType: PropertyType) => {
    setEditProperties((prev) => prev.map((p, i) => (i === index ? { ...p, type: newType } : p)));
  };

  const updatePropertySensitivity = (index: number, level: SensitivityLevel) => {
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
    // 新建模板直接删除，无需检查
    if (isNewTemplate) {
      setEditProperties((prev) => prev.filter((_, i) => i !== index));
      return;
    }
    try {
      const usage = await checkFieldUsage(editingTemplate.id, prop.id);
      if (usage.active > 0 || usage.softDeleted > 0) {
        const confirmed = confirm(
          `${t('settings:confirm_deprecate_title')}\n\n${t('settings:confirm_deprecate_body', { activeCount: usage.active, softDeletedCount: usage.softDeleted })}`,
        );
        if (confirmed) {
          setEditProperties((prev) =>
            prev.map((p, i) =>
              i === index ? { ...p, deprecatedAt: new Date().toISOString() } : p,
            ),
          );
          setFieldUsageMap((prev) => ({ ...prev, [prop.id]: usage }));
        }
      } else {
        setEditProperties((prev) => prev.filter((_, i) => i !== index));
      }
    } catch {
      // 检查失败时保守处理：标记为废弃
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

  const renderTypeSelect = (value: PropertyType, onChange: (v: PropertyType) => void) => (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value as PropertyType)}
      style={{
        height: 36,
        padding: '0 10px',
        borderRadius: 6,
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-elevated)',
        color: 'var(--text-primary)',
        fontSize: 13,
        cursor: 'pointer',
        boxSizing: 'border-box',
      }}
    >
      {PROPERTY_TYPES.map((pt) => (
        <option key={pt} value={pt}>
          {t(`editor:field_types.${pt}`, pt)}
        </option>
      ))}
    </select>
  );

  const renderPageSelect = (value: string, onChange: (v: string) => void, label?: string) => {
    const customPages = settings.customPages || [];
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {label && <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{label}</label>}
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={{
            padding: '8px 10px',
            borderRadius: 6,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            fontSize: 13,
            cursor: 'pointer',
          }}
        >
          <optgroup label={t('settings:system_pages') || '系统页面'}>
            {SYSTEM_PAGES.map((cat) => (
              <option key={cat} value={cat}>
                {t(`navigation:${cat}`, cat)}
              </option>
            ))}
          </optgroup>
          {customPages.filter((p) => !p.deletedAt).length > 0 && (
            <optgroup label={t('settings:custom_pages') || '自定义页面'}>
              {customPages
                .filter((p) => !p.deletedAt)
                .map((page) => (
                  <option key={page.id} value={page.id}>
                    {page.name}
                  </option>
                ))}
            </optgroup>
          )}
          {customPages.filter((p) => p.deletedAt).length > 0 && (
            <optgroup label={t('settings:custom_pages_trash') || '自定义页面（回收站）'}>
              {customPages
                .filter((p) => p.deletedAt)
                .map((page) => (
                  <option key={page.id} value={page.id} disabled>
                    {page.name}
                  </option>
                ))}
            </optgroup>
          )}
          {value &&
            !SYSTEM_PAGES.includes(value as (typeof SYSTEM_PAGES)[number]) &&
            !customPages.find((p) => p.id === value) && (
              <option value={value} disabled>
                {t('settings:deleted_page') || '（页面已删除）'}
              </option>
            )}
        </select>
      </div>
    );
  };

  const from = (location.state as { from?: string } | null)?.from;
  const handleBack = () => {
    if (from && from.startsWith('/editor')) {
      // Pop the TemplateManager entry so the editor page remains the only one.
      // Using replace would duplicate the editor entry, forcing two Back clicks.
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
            <BookOpen size={16} style={{ marginRight: 4 }} />
            {t('settings:sample_templates') || '模板示例'}
          </Button>
          <Button onClick={openCreate}>
            <Plus size={16} style={{ marginRight: 4 }} />
            {t('settings:new_template') || '新建模板'}
          </Button>
        </div>
      }
    >
      <div
        style={{
          maxWidth: 700,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        {isLoading && <LoadingPlaceholder variant="base" minHeight={120} />}
        {error && <div style={{ color: 'var(--error)' }}>{error}</div>}

        {!isLoading && !error && allTemplates.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <Input
              placeholder={t('settings:search_templates') || '搜索模板...'}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
              {pageOptions.map((opt) => (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => setPageFilter(opt.id)}
                  aria-pressed={pageFilter === opt.id}
                  style={{
                    padding: '5px 12px',
                    borderRadius: 6,
                    border: '1px solid var(--border-subtle)',
                    background: pageFilter === opt.id ? 'var(--accent-primary)' : 'transparent',
                    color: pageFilter === opt.id ? 'white' : 'var(--text-secondary)',
                    fontSize: 12,
                    cursor: 'pointer',
                  }}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>
        )}

        {!isLoading && allTemplates.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
            <LayoutTemplate size={48} style={{ marginBottom: 12, opacity: 0.4 }} />
            <div>{t('settings:no_templates') || '暂无模板'}</div>
            <div style={{ fontSize: 12, marginTop: 4 }}>
              {t('settings:no_templates_hint') || '点击右上角"新建模板"创建'}
            </div>
          </div>
        )}

        {!isLoading && allTemplates.length > 0 && filteredTemplates.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
            <div>{t('settings:no_templates_filtered') || '没有符合筛选条件的模板'}</div>
          </div>
        )}

        {filteredTemplates.map((tpl) => (
          <Card key={tpl.id} interactive onClick={() => setDetailTemplate(tpl)}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <LayoutTemplate size={20} />
                <div>
                  <div
                    style={{
                      fontSize: 14,
                      fontWeight: 500,
                      display: 'flex',
                      alignItems: 'center',
                      gap: 6,
                    }}
                  >
                    {tpl.name}
                  </div>
                  <div
                    style={{
                      fontSize: 11,
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
                  <Pencil size={16} />
                </Button>
                <Button
                  variant="tertiary"
                  size="sm"
                  onClick={() => handleDelete(tpl.id, tpl.name)}
                  style={{ color: '#e74c3c' }}
                >
                  <Trash2 size={16} />
                </Button>
              </div>
            </div>
          </Card>
        ))}
      </div>

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
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
          <Input
            label={t('common:name') || '名称'}
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
          />
          {renderPageSelect(
            editCategory,
            setEditCategory,
            t('settings:template_category') || '所属页面',
          )}

          <div>
            {/* Active fields */}
            <div
              style={{
                fontSize: 13,
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
                  <div style={{ fontSize: 12, color: 'var(--text-tertiary)', padding: '12px 0' }}>
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
                  .map(({ prop, idx }) => (
                      <div
                        key={prop.id}
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
                            onChange={(e) => updatePropertyName(idx, e.target.value)}
                            placeholder={t('settings:field_name') || '字段名称'}
                            style={{
                              width: '100%',
                              height: 36,
                              padding: '0 10px',
                              borderRadius: 6,
                              border: '1px solid var(--border-subtle)',
                              background: 'var(--bg-elevated)',
                              color: 'var(--text-primary)',
                              fontSize: 14,
                              fontFamily: 'inherit',
                              outline: 'none',
                              boxSizing: 'border-box',
                            }}
                          />
                        </div>
                        {(() => {
                          const sharedSelectStyle: React.CSSProperties = {
                            height: 36,
                            padding: '0 10px',
                            borderRadius: 6,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-elevated)',
                            color: 'var(--text-primary)',
                            fontSize: 13,
                            cursor: 'pointer',
                            boxSizing: 'border-box',
                          };
                          return (
                            <select
                              value={prop.type}
                              onChange={(e) =>
                                updatePropertyType(idx, e.target.value as PropertyType)
                              }
                              style={{ ...sharedSelectStyle, minWidth: 90 }}
                            >
                              {PROPERTY_TYPES.map((pt) => (
                                <option key={pt} value={pt}>
                                  {t(`editor:field_types.${pt}`, pt)}
                                </option>
                              ))}
                            </select>
                          );
                        })()}
                        {(prop.type === 'select' || prop.type === 'multiselect') && (
                          <OptionsEditor
                            options={prop.options || []}
                            onChange={(opts) => updatePropertyOptions(idx, opts)}
                            fieldName={prop.name}
                            fieldType={prop.type === 'multiselect' ? 'multiselect' : 'select'}
                          />
                        )}
                        <select
                          value={prop.sensitivityLevel || 'internal'}
                          onChange={(e) =>
                            updatePropertySensitivity(idx, e.target.value as SensitivityLevel)
                          }
                          style={{
                            height: 36,
                            padding: '0 10px',
                            borderRadius: 6,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-elevated)',
                            color: 'var(--text-primary)',
                            fontSize: 13,
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
                        <button
                          type="button"
                          onClick={() => removeProperty(idx)}
                          title={t('settings:remove_field') || '删除'}
                          style={{
                            height: 36,
                            width: 36,
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            borderRadius: 6,
                            border: '1px solid var(--border-subtle)',
                            background: 'transparent',
                            color: '#e74c3c',
                            cursor: 'pointer',
                            flexShrink: 0,
                          }}
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                  ))}
              </div>

              {/* Deprecated fields */}
              {editProperties.filter((p) => p.deprecatedAt).length > 0 && (
                <div style={{ marginTop: 16 }}>
                  <button
                    onClick={() => setShowDeprecated((v) => !v)}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 6,
                      fontSize: 12,
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
                      count: editProperties.filter((p) => p.deprecatedAt).length,
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
                      {editProperties
                        .map((prop, idx) => ({ prop, idx }))
                        .filter(({ prop }) => prop.deprecatedAt)
                        .map(({ prop, idx }) => {
                          const usage = fieldUsageMap[prop.id];
                          const cleanable = usage
                            ? usage.active === 0 && usage.softDeleted === 0
                            : false;
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
                              {/* Row 1: field info + action buttons */}
                              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                                <span
                                  style={{
                                    color: 'var(--text-tertiary)',
                                    display: 'flex',
                                    alignItems: 'center',
                                  }}
                                >
                                  {FIELD_TYPE_ICONS[prop.type] || FIELD_TYPE_ICONS.text}
                                </span>
                                <span
                                  style={{
                                    fontSize: 14,
                                    fontWeight: 500,
                                    color: 'var(--text-secondary)',
                                    flex: 1,
                                    minWidth: 0,
                                    textDecoration: 'line-through',
                                  }}
                                >
                                  {prop.name}
                                </span>
                                <UiSensitivityBadge
                                  level={(prop.sensitivityLevel || 'internal') as SensitivityLevel}
                                />
                                <DeprecatedBadge />
                                <Button
                                  variant="tertiary"
                                  size="sm"
                                  onClick={() => restoreProperty(idx)}
                                >
                                  {t('common:restore') || '恢复'}
                                </Button>
                                {cleanable && (
                                  <Button
                                    variant="tertiary"
                                    size="sm"
                                    onClick={() => permanentlyRemoveProperty(idx)}
                                    style={{ color: '#e74c3c' }}
                                  >
                                    {t('common:clean_up') || '清理'}
                                  </Button>
                                )}
                              </div>
                              {/* Row 2: usage hint + go-to-trash link */}
                              {usage && !cleanable && (
                                <div
                                  style={{
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: 8,
                                    justifyContent: 'space-between',
                                  }}
                                >
                                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
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
                                      onClick={() => navigate('/settings/trash')}
                                      style={{
                                        fontSize: 11,
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
              )}
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 12 }}>
              {renderTypeSelect(newFieldType, setNewFieldType)}
              <button
                type="button"
                onClick={addProperty}
                style={{
                  height: 36,
                  padding: '0 14px',
                  borderRadius: 6,
                  border: 'none',
                  background: 'var(--accent-primary)',
                  color: 'white',
                  fontSize: 13,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  whiteSpace: 'nowrap',
                }}
              >
                <Plus size={14} />
                {t('settings:add_field') || '添加字段'}
              </button>
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
            <Button variant="secondary" onClick={closeEdit}>
              <X size={16} style={{ marginRight: 4 }} />
              {t('common:cancel') || '取消'}
            </Button>
            <Button onClick={saveEdit}>
              <Save size={16} style={{ marginRight: 4 }} />
              {t('common:save') || '保存'}
            </Button>
          </div>
        </div>
      </Dialog>

      {/* Template detail modal */}
      {detailTemplate && (
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
          onClick={() => setDetailTemplate(null)}
        >
          <div
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
                <LayoutTemplate size={24} color="var(--accent-primary)" />
                <div>
                  <h2 style={{ fontSize: 18, fontWeight: 700, margin: 0 }}>
                    {detailTemplate.name}
                  </h2>
                  <span
                    style={{
                      fontSize: 11,
                      color: 'var(--text-tertiary)',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                    }}
                  >
                    {(() => {
                      const page = resolvePageLabel(detailTemplate.category || 'identity');
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
                      {detailTemplate.properties.length} {t('settings:template_fields') || '个字段'}
                    </span>
                    <SensitivityBadges properties={detailTemplate.properties} />
                  </span>
                </div>
              </div>
              <button
                onClick={() => setDetailTemplate(null)}
                style={{
                  padding: 6,
                  borderRadius: 8,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: 'var(--text-tertiary)',
                }}
              >
                <X size={20} />
              </button>
            </div>

            {/* Divider */}
            <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 20 }} />

            {/* Fields */}
            {detailTemplate.properties.length === 0 ? (
              <p
                style={{
                  fontSize: 13,
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
                          {FIELD_TYPE_ICONS[prop.type] || FIELD_TYPE_ICONS.text}
                        </span>
                        <span
                          style={{
                            fontSize: 14,
                            fontWeight: 500,
                            color: 'var(--text-primary)',
                            textDecoration: prop.deprecatedAt ? 'line-through' : 'none',
                          }}
                        >
                          {prop.name}
                        </span>
                      </div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        <UiSensitivityBadge
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
              <Button variant="secondary" onClick={() => setDetailTemplate(null)}>
                {t('common:close') || '关闭'}
              </Button>
              <Button
                onClick={() => {
                  const ut = templates.find((u) => u.id === detailTemplate.id);
                  if (ut) {
                    setDetailTemplate(null);
                    openEdit(ut);
                  }
                }}
              >
                <Pencil size={16} style={{ marginRight: 4 }} />
                {t('common:edit') || '编辑'}
              </Button>
            </div>
          </div>
        </div>
      )}

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
                })),
              );
              setSelectedSample(null);
              setShowSampleGallery(false);
            } catch (e) {
              alert(t('common:save_failed') + ': ' + e);
            }
          }}
        />
      )}

      {/* Delete confirmation dialog */}
      {confirmDelete && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 9999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.4)',
          }}
          onClick={() => setConfirmDelete(null)}
        >
          <div
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 12,
              padding: '24px 28px',
              maxWidth: 360,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
              {t('settings:template_delete_confirm_title')}
            </h3>
            <p
              style={{
                margin: '0 0 20px',
                fontSize: 14,
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('settings:template_delete_confirm_body', { name: confirmDelete.name })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <Button variant="secondary" onClick={() => setConfirmDelete(null)}>
                {t('common:cancel') || '取消'}
              </Button>
              <Button
                onClick={doDelete}
                style={{ background: '#e74c3c', color: 'white', borderColor: '#e74c3c' }}
              >
                {t('common:delete') || '删除'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </AppShell>
  );
}
