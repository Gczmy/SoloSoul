import { useState, useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Input } from '@/components/ui/Input';
import { useTemplateStore } from '@/stores/templateStore';
import {
  LayoutTemplate, Trash2, Pencil, X, Save, Plus,
  Type, AlignLeft, Hash, Calendar, Clock, CheckSquare,
  List, ListChecks, Link, Mail, Phone, File,
} from 'lucide-react';
import { SensitivityBadge as UiSensitivityBadge } from '@/components/ui/SensitivityBadge';
import type { UserTemplate, TemplateProperty, PropertyType, SensitivityLevel } from '@/types/template';

const PROPERTY_TYPES: PropertyType[] = [
  'text', 'multiline', 'number', 'date', 'datetime',
  'boolean', 'select', 'multiselect', 'url', 'email', 'phone', 'file',
];

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

const TEMPLATE_CATEGORIES = ['identity', 'travel', 'financial', 'professional'] as const;
type TemplateCategory = typeof TEMPLATE_CATEGORIES[number];

interface ListTemplate {
  id: string;
  name: string;
  category: string;
  properties: Array<{ id: string; name: string; type: string; sensitivityLevel?: string }>;
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
  } = useTemplateStore();

  const [editingTemplate, setEditingTemplate] = useState<UserTemplate | null>(null);
  const [isNewTemplate, setIsNewTemplate] = useState(false);
  const [editName, setEditName] = useState('');
  const [editCategory, setEditCategory] = useState<TemplateCategory>('identity');
  const [editProperties, setEditProperties] = useState<TemplateProperty[]>([]);

  const [newFieldType, setNewFieldType] = useState<PropertyType>('text');
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [detailTemplate, setDetailTemplate] = useState<ListTemplate | null>(null);

  useEffect(() => {
    loadTemplates().catch(() => {});
  }, [loadTemplates]);

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
      })),
    }));
  }, [templates]);

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
    setEditCategory((tpl.category as TemplateCategory) || 'identity');
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

  const removeProperty = (index: number) => {
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

  const renderCategorySelect = (value: TemplateCategory, onChange: (v: TemplateCategory) => void, label?: string) => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {label && <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{label}</label>}
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as TemplateCategory)}
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
        {TEMPLATE_CATEGORIES.map((cat) => (
          <option key={cat} value={cat}>{t(`navigation:${cat}`, cat)}</option>
        ))}
      </select>
    </div>
  );

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
          {renderCategorySelect(editCategory, setEditCategory, t('settings:template_category') || '所属分类')}

          <div>
            <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 8, color: 'var(--text-secondary)' }}>
              {t('settings:fields_section_title') || '字段列表'}
            </div>

            {editProperties.length === 0 && (
              <div style={{ fontSize: 12, color: 'var(--text-tertiary)', padding: '12px 0' }}>
                {t('settings:empty_template_hint') || '此模板暂无字段，点击下方添加'}
              </div>
            )}

            <div style={{ display: 'flex', flexDirection: 'column', gap: 8, maxHeight: '45vh', overflow: 'auto', paddingRight: 4 }}>
              {editProperties.map((prop, idx) => (
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
              ))}
            </div>

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
                    <span>{t(`navigation:${detailTemplate.category || 'identity'}`, detailTemplate.category || 'identity')} · {detailTemplate.properties.length} {t('settings:template_fields') || '个字段'}</span>
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
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
                        <span style={{ color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center' }}>
                          {iconMap[prop.type] || iconMap.text}
                        </span>
                        <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-primary)' }}>{prop.name}</span>
                      </div>
                      <UiSensitivityBadge level={(prop.sensitivityLevel || 'internal') as SensitivityLevel} />
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
