import { useState, useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Input } from '@/components/ui/Input';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import {
  LayoutTemplate, Trash2, Pencil, X, Save, Plus,
  Type, AlignLeft, Hash, Calendar, Clock, CheckSquare,
  List, ListChecks, Link, Mail, Phone, File,
} from 'lucide-react';
import { SensitivityBadge as UiSensitivityBadge } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import type { UserTemplate, TemplateProperty, PropertyType, SensitivityLevel } from '@/types/template';

const PROPERTY_TYPES: PropertyType[] = [
  'text', 'multiline', 'number', 'date', 'datetime',
  'boolean', 'select', 'multiselect', 'url', 'email', 'phone', 'file',
];

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

const SYSTEM_PAGES = ['identity', 'travel', 'financial', 'professional'] as const;

interface ListTemplate {
  id: string;
  name: string;
  category: string;
  properties: Array<{ id: string; name: string; type: string; sensitivityLevel?: string; deprecatedAt?: string }>;
}

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

function SensitivityBadges({ properties }: { properties: ListTemplate['properties'] }) {
  const present = new Set(properties.map((p) => (p.sensitivityLevel || 'internal') as SensitivityLevel));
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
  const { t } = useTranslation(['common', 'settings', 'editor']);
  const {
    templates, isLoading, error, loadTemplates,
    deleteTemplate, updateTemplate, createTemplate,
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
  const [fieldUsageMap, setFieldUsageMap] = useState<Record<string, { active: number; softDeleted: number }>>({});

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
          } catch { /* ignore */ }
        })
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
    if (SYSTEM_PAGES.includes(category as typeof SYSTEM_PAGES[number])) {
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
    setEditProperties((prev) =>
      prev.map((p, i) => (i === index ? { ...p, name: newName } : p))
    );
  };

  const updatePropertyType = (index: number, newType: PropertyType) => {
    setEditProperties((prev) =>
      prev.map((p, i) => (i === index ? { ...p, type: newType } : p))
    );
  };

  const updatePropertySensitivity = (index: number, level: SensitivityLevel) => {
    setEditProperties((prev) =>
      prev.map((p, i) => (i === index ? { ...p, sensitivityLevel: level } : p))
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
          `${t('settings:confirm_deprecate_title')}\n\n${t('settings:confirm_deprecate_body', { activeCount: usage.active, softDeletedCount: usage.softDeleted })}`
        );
        if (confirmed) {
          setEditProperties((prev) =>
            prev.map((p, i) => (i === index ? { ...p, deprecatedAt: new Date().toISOString() } : p))
          );
          setFieldUsageMap((prev) => ({ ...prev, [prop.id]: usage }));
        }
      } else {
        setEditProperties((prev) => prev.filter((_, i) => i !== index));
      }
    } catch {
      // 检查失败时保守处理：标记为废弃
      setEditProperties((prev) =>
        prev.map((p, i) => (i === index ? { ...p, deprecatedAt: new Date().toISOString() } : p))
      );
    }
  };

  const restoreProperty = (index: number) => {
    setEditProperties((prev) =>
      prev.map((p, i) => (i === index ? { ...p, deprecatedAt: undefined } : p))
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
        padding: '8px 10px',
        borderRadius: 6,
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-elevated)',
        color: 'var(--text-primary)',
        fontSize: 13,
        cursor: 'pointer',
      }}
    >
      {PROPERTY_TYPES.map((pt) => (
        <option key={pt} value={pt}>{t(`editor:field_types.${pt}`, pt)}</option>
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
              <option key={cat} value={cat}>{t(`navigation:${cat}`, cat)}</option>
            ))}
          </optgroup>
          {customPages.filter((p) => !p.deletedAt).length > 0 && (
            <optgroup label={t('settings:custom_pages') || '自定义页面'}>
              {customPages.filter((p) => !p.deletedAt).map((page) => (
                <option key={page.id} value={page.id}>{page.name}</option>
              ))}
            </optgroup>
          )}
          {customPages.filter((p) => p.deletedAt).length > 0 && (
            <optgroup label={t('settings:custom_pages_trash') || '自定义页面（回收站）'}>
              {customPages.filter((p) => p.deletedAt).map((page) => (
                <option key={page.id} value={page.id} disabled>{page.name}</option>
              ))}
            </optgroup>
          )}
          {value && !SYSTEM_PAGES.includes(value as typeof SYSTEM_PAGES[number]) && !customPages.find((p) => p.id === value) && (
            <option value={value} disabled>{t('settings:deleted_page') || '（页面已删除）'}</option>
          )}
        </select>
      </div>
    );
  };

  return (
    <AppShell
      title={t('settings:template_manager_title') || '模板管理'}
      onBack={() => navigate('/settings')}
      actions={
        <Button onClick={openCreate}>
          <Plus size={16} style={{ marginRight: 4 }} />
          {t('settings:new_template') || '新建模板'}
        </Button>
      }
    >
      <div style={{ maxWidth: 700, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {isLoading && <div style={{ textAlign: 'center', color: 'var(--text-secondary)' }}>{t('common:loading')}</div>}
        {error && <div style={{ color: 'var(--error)' }}>{error}</div>}

        {!isLoading && allTemplates.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
            <LayoutTemplate size={48} style={{ marginBottom: 12, opacity: 0.4 }} />
            <div>{t('settings:no_templates') || '暂无模板'}</div>
            <div style={{ fontSize: 12, marginTop: 4 }}>{t('settings:no_templates_hint') || '点击右上角"新建模板"创建'}</div>
          </div>
        )}

        {allTemplates.map((tpl) => (
          <Card key={tpl.id} interactive onClick={() => setDetailTemplate(tpl)}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <LayoutTemplate size={20} />
                <div>
                  <div style={{ fontSize: 14, fontWeight: 500, display: 'flex', alignItems: 'center', gap: 6 }}>
                    {tpl.name}
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
                    {(() => {
                      const page = resolvePageLabel(tpl.category);
                      return (
                        <span style={page.deleted ? { textDecoration: 'line-through', opacity: 0.6 } : undefined}>
                          {page.name}
                        </span>
                      );
                    })()}
                    <span>·</span>
                    <span>{tpl.properties.length} {t('settings:template_fields') || '个字段'}</span>
                    <SensitivityBadges properties={tpl.properties} />
                  </div>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 8 }} onClick={(e) => e.stopPropagation()}>
                <Button variant="tertiary" size="sm" onClick={() => {
                  const ut = templates.find((u) => u.id === tpl.id);
                  if (ut) openEdit(ut);
                }}>
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
      <Dialog isOpen={!!editingTemplate} onClose={closeEdit} title={isNewTemplate ? (t('settings:new_template') || '新建模板') : (t('settings:edit_template') || '编辑模板')}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
          <Input
            label={t('common:name') || '名称'}
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
          />
          {renderPageSelect(editCategory, setEditCategory, t('settings:template_category') || '所属页面')}

          <div>
            {/* Active fields */}
            <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 8, color: 'var(--text-secondary)' }}>
              {t('settings:fields_section_title') || '字段列表'}
            </div>

            {editProperties.filter((p) => !p.deprecatedAt).length === 0 && editProperties.filter((p) => p.deprecatedAt).length === 0 && (
              <div style={{ fontSize: 12, color: 'var(--text-tertiary)', padding: '12px 0' }}>
                {t('settings:empty_template_hint') || '此模板暂无字段，点击下方添加'}
              </div>
            )}

            <div style={{ display: 'flex', flexDirection: 'column', gap: 8, maxHeight: '35vh', overflow: 'auto', paddingRight: 4 }}>
              {editProperties.filter((p) => !p.deprecatedAt).map((prop) => {
                const idx = editProperties.findIndex((p) => p.id === prop.id);
                return (
                  <div
                    key={prop.id}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 6,
                      padding: '8px 10px',
                      borderRadius: 6,
                      background: 'var(--bg-subtle)',
                      flexWrap: 'wrap',
                    }}
                  >
                    <div style={{ flex: 1, minWidth: 120 }}>
                      <input
                        value={prop.name}
                        onChange={(e) => updatePropertyName(idx, e.target.value)}
                        placeholder={t('settings:field_name') || '字段名称'}
                        style={{
                          width: '100%',
                          padding: '8px 10px',
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
                    {renderTypeSelect(prop.type, (v) => updatePropertyType(idx, v))}
                    <select
                      value={prop.sensitivityLevel || 'internal'}
                      onChange={(e) => updatePropertySensitivity(idx, e.target.value as SensitivityLevel)}
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
                      {SENSITIVITY_LEVELS.map((sl) => (
                        <option key={sl} value={sl}>{t(`editor:sensitivity_levels.${sl}`, sl)}</option>
                      ))}
                    </select>
                    <Button variant="tertiary" size="sm" onClick={() => removeProperty(idx)} title={t('settings:remove_field') || '删除'}>
                      <Trash2 size={14} />
                    </Button>
                  </div>
                );
              })}
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
                  <span style={{ transform: showDeprecated ? 'rotate(90deg)' : 'rotate(0deg)', transition: 'transform 0.15s ease', display: 'inline-block' }}>▶</span>
                  {t('settings:deprecated_fields_count', { count: editProperties.filter((p) => p.deprecatedAt).length })}
                </button>
                {showDeprecated && (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 8, maxHeight: '20vh', overflow: 'auto', paddingRight: 4 }}>
                    {editProperties.filter((p) => p.deprecatedAt).map((prop) => {
                      const idx = editProperties.findIndex((p) => p.id === prop.id);
                      const usage = fieldUsageMap[prop.id];
                      const cleanable = usage ? usage.active === 0 && usage.softDeleted === 0 : false;
                      const iconMap: Record<string, React.ReactNode> = {
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
                            <span style={{ color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center' }}>
                              {iconMap[prop.type] || iconMap.text}
                            </span>
                            <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-secondary)', flex: 1, minWidth: 0, textDecoration: 'line-through' }}>
                              {prop.name}
                            </span>
                            <UiSensitivityBadge level={(prop.sensitivityLevel || 'internal') as SensitivityLevel} />
                            <DeprecatedBadge />
                            <Button variant="tertiary" size="sm" onClick={() => restoreProperty(idx)}>
                              {t('common:restore') || '恢复'}
                            </Button>
                            {cleanable && (
                              <Button variant="tertiary" size="sm" onClick={() => permanentlyRemoveProperty(idx)} style={{ color: '#e74c3c' }}>
                                {t('common:clean_up') || '清理'}
                              </Button>
                            )}
                          </div>
                          {/* Row 2: usage hint + go-to-trash link */}
                          {usage && !cleanable && (
                            <div style={{ display: 'flex', alignItems: 'center', gap: 8, justifyContent: 'space-between' }}>
                              <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                                {usage.active > 0 && usage.softDeleted > 0
                                  ? t('settings:field_in_use_both', { activeCount: usage.active, softDeletedCount: usage.softDeleted })
                                  : usage.active > 0
                                  ? t('settings:field_in_use_active', { activeCount: usage.active })
                                  : t('settings:field_in_use_trash', { softDeletedCount: usage.softDeleted })}
                              </span>
                              {usage.softDeleted > 0 && (
                                <span
                                  onClick={() => navigate('/settings/trash')}
                                  style={{ fontSize: 11, color: 'var(--accent-primary)', cursor: 'pointer', textDecoration: 'underline', whiteSpace: 'nowrap' }}
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

            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 12, flexWrap: 'wrap' }}>
              {renderTypeSelect(newFieldType, setNewFieldType)}
              <Button variant="secondary" size="sm" onClick={addProperty}>
                <Plus size={14} style={{ marginRight: 4 }} />
                {t('settings:add_field') || '添加字段'}
              </Button>
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
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 20 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <LayoutTemplate size={24} color="var(--accent-primary)" />
                <div>
                  <h2 style={{ fontSize: 18, fontWeight: 700, margin: 0 }}>{detailTemplate.name}</h2>
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center', gap: 8 }}>
                    {(() => {
                      const page = resolvePageLabel(detailTemplate.category || 'identity');
                      return (
                        <span style={page.deleted ? { textDecoration: 'line-through', opacity: 0.6 } : undefined}>
                          {page.name}
                        </span>
                      );
                    })()}
                    <span>·</span>
                    <span>{detailTemplate.properties.length} {t('settings:template_fields') || '个字段'}</span>
                    <SensitivityBadges properties={detailTemplate.properties} />
                  </span>
                </div>
              </div>
              <button
                onClick={() => setDetailTemplate(null)}
                style={{ padding: 6, borderRadius: 8, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}
              >
                <X size={20} />
              </button>
            </div>

            {/* Divider */}
            <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 20 }} />

            {/* Fields */}
            {detailTemplate.properties.length === 0 ? (
              <p style={{ fontSize: 13, color: 'var(--text-tertiary)', textAlign: 'center', padding: '16px 0' }}>
                {t('settings:empty_template_hint') || '此模板暂无字段'}
              </p>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                {detailTemplate.properties.map((prop) => {
                  const iconMap: Record<string, React.ReactNode> = {
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
                  return (
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
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
                        <span style={{ color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center' }}>
                          {iconMap[prop.type] || iconMap.text}
                        </span>
                        <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-primary)', textDecoration: prop.deprecatedAt ? 'line-through' : 'none' }}>{prop.name}</span>
                      </div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        <UiSensitivityBadge level={(prop.sensitivityLevel || 'internal') as SensitivityLevel} />
                        {prop.deprecatedAt && <DeprecatedBadge />}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Actions */}
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 24 }}>
              <Button variant="secondary" onClick={() => setDetailTemplate(null)}>
                {t('common:close') || '关闭'}
              </Button>
              <Button onClick={() => {
                const ut = templates.find((u) => u.id === detailTemplate.id);
                if (ut) { setDetailTemplate(null); openEdit(ut); }
              }}>
                <Pencil size={16} style={{ marginRight: 4 }} />
                {t('common:edit') || '编辑'}
              </Button>
            </div>
          </div>
        </div>
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
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('settings:template_delete_confirm_body', { name: confirmDelete.name })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <Button variant="secondary" onClick={() => setConfirmDelete(null)}>
                {t('common:cancel') || '取消'}
              </Button>
              <Button onClick={doDelete} style={{ background: '#e74c3c', color: 'white', borderColor: '#e74c3c' }}>
                {t('common:delete') || '删除'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </AppShell>
  );
}
