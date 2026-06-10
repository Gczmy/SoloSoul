import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Input } from '@/components/ui/Input';
import { useTemplateStore } from '@/stores/templateStore';
import { LayoutTemplate, Trash2, Pencil, X, Save, Plus } from 'lucide-react';
import type { UserTemplate, TemplateProperty, PropertyType, SensitivityLevel } from '@/types/template';

const PROPERTY_TYPES: PropertyType[] = [
  'text', 'multiline', 'number', 'date', 'datetime',
  'boolean', 'select', 'multiselect', 'url', 'email', 'phone', 'file',
];

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

export function TemplateManagerPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'settings', 'editor']);
  const {
    templates, isLoading, error, loadTemplates,
    deleteTemplate, updateTemplate, createTemplate, getTemplate,
  } = useTemplateStore();

  const [editingTemplate, setEditingTemplate] = useState<UserTemplate | null>(null);
  const [editName, setEditName] = useState('');
  const [editProperties, setEditProperties] = useState<TemplateProperty[]>([]);

  const [isCreating, setIsCreating] = useState(false);
  const [createName, setCreateName] = useState('');

  const [newFieldType, setNewFieldType] = useState<PropertyType>('text');

  useEffect(() => {
    loadTemplates().catch(() => {});
  }, [loadTemplates]);

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(t('common:confirm_delete_template', { name }) || `确定要删除模板 "${name}" 吗？此操作不可撤销。`)) {
      return;
    }
    try {
      await deleteTemplate(id);
    } catch (e) {
      alert(t('common:delete_failed') + ': ' + e);
    }
  };

  const openEdit = (tpl: UserTemplate) => {
    setEditingTemplate(tpl);
    setEditName(tpl.name);
    setEditProperties([...tpl.properties]);
  };

  const closeEdit = () => {
    setEditingTemplate(null);
    setEditName('');
    setEditProperties([]);
  };

  const saveEdit = async () => {
    if (!editingTemplate) return;
    try {
      await updateTemplate(editingTemplate.id, {
        name: editName.trim() || editingTemplate.name,
        properties: editProperties,
      });
      closeEdit();
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

  const closeCreate = () => {
    setIsCreating(false);
    setCreateName('');
  };

  const handleCreate = async () => {
    const name = createName.trim();
    if (!name) return;
    try {
      const newId = await createTemplate(name, undefined, []);
      closeCreate();
      const tpl = await getTemplate(newId);
      if (tpl) openEdit(tpl);
    } catch (e) {
      alert(t('common:create_failed') + ': ' + e);
    }
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

  return (
    <AppShell
      title={t('settings:template_manager_title') || '模板管理'}
      onBack={() => navigate('/settings')}
      actions={
        <Button onClick={() => setIsCreating(true)}>
          <Plus size={16} style={{ marginRight: 4 }} />
          {t('settings:new_template') || '新建模板'}
        </Button>
      }
    >
      <div style={{ maxWidth: 700, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {isLoading && <div style={{ textAlign: 'center', color: 'var(--text-secondary)' }}>{t('common:loading')}</div>}
        {error && <div style={{ color: 'var(--error)' }}>{error}</div>}

        {!isLoading && templates.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
            <LayoutTemplate size={48} style={{ marginBottom: 12, opacity: 0.4 }} />
            <div>{t('settings:no_templates') || '暂无自定义模板'}</div>
            <div style={{ fontSize: 12, marginTop: 4 }}>{t('settings:no_templates_hint') || '点击右上角"新建模板"创建'}</div>
          </div>
        )}

        {templates.map((tpl) => (
          <Card key={tpl.id}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <LayoutTemplate size={20} />
                <div>
                  <div style={{ fontSize: 14, fontWeight: 500 }}>{tpl.name}</div>
                  <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                    {tpl.properties.length} {t('settings:template_fields') || '个字段'}
                    {tpl.properties.some((p) => (p.sensitivityLevel || 'internal') !== 'public') && ` · ${t('settings:has_sensitive') || '含敏感字段'}`}
                  </div>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <Button variant="tertiary" size="sm" onClick={() => openEdit(tpl)}>
                  <Pencil size={16} />
                </Button>
                <Button variant="tertiary" size="sm" onClick={() => handleDelete(tpl.id, tpl.name)}>
                  <Trash2 size={16} />
                </Button>
              </div>
            </div>
          </Card>
        ))}
      </div>

      {/* Create Dialog */}
      <Dialog isOpen={isCreating} onClose={closeCreate} title={t('settings:new_template') || '新建模板'}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <Input
            label={t('common:name') || '名称'}
            value={createName}
            onChange={(e) => setCreateName(e.target.value)}
            placeholder={t('settings:template_name_placeholder') || '请输入模板名称'}
            autoFocus
          />
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
            <Button variant="secondary" onClick={closeCreate}>
              <X size={16} style={{ marginRight: 4 }} />
              {t('common:cancel') || '取消'}
            </Button>
            <Button onClick={handleCreate} disabled={!createName.trim()}>
              <Plus size={16} style={{ marginRight: 4 }} />
              {t('common:create') || '创建'}
            </Button>
          </div>
        </div>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog isOpen={!!editingTemplate} onClose={closeEdit} title={t('settings:edit_template') || '编辑模板'}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
          <Input
            label={t('common:name') || '名称'}
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
          />

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
    </AppShell>
  );
}
