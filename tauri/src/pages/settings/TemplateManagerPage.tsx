import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Input } from '@/components/ui/Input';
import { useTemplateStore } from '@/stores/templateStore';
import { TemplateFieldInput } from '@/components/TemplateFieldInput';
import { LayoutTemplate, Trash2, Pencil, X, Save } from 'lucide-react';
import type { UserTemplate, TemplateProperty } from '@/types/template';

export function TemplateManagerPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'settings']);
  const { templates, isLoading, error, loadTemplates, deleteTemplate, updateTemplate } = useTemplateStore();

  const [editingTemplate, setEditingTemplate] = useState<UserTemplate | null>(null);
  const [editName, setEditName] = useState('');
  const [editProperties, setEditProperties] = useState<TemplateProperty[]>([]);

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

  return (
    <AppShell title={t('settings:template_manager_title') || '模板管理'} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 700, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {isLoading && <div style={{ textAlign: 'center', color: 'var(--text-secondary)' }}>{t('common:loading')}</div>}
        {error && <div style={{ color: 'var(--error)' }}>{error}</div>}

        {!isLoading && templates.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
            <LayoutTemplate size={48} style={{ marginBottom: 12, opacity: 0.4 }} />
            <div>{t('settings:no_templates') || '暂无自定义模板'}</div>
            <div style={{ fontSize: 12, marginTop: 4 }}>{t('settings:no_templates_hint') || '在对象编辑页点击"保存为模板"即可创建'}</div>
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
                    {tpl.properties.some((p) => p.sensitive) && ` · ${t('settings:has_sensitive') || '含敏感字段'}`}
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

      {/* Edit Dialog */}
      <Dialog isOpen={!!editingTemplate} onClose={closeEdit} title={t('settings:edit_template') || '编辑模板'}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16, minWidth: 400 }}>
          <Input
            label={t('common:name') || '名称'}
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
          />

          <div>
            <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 8, color: 'var(--text-secondary)' }}>
              {t('settings:template_fields') || '字段'}
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              {editProperties.map((prop, idx) => (
                <div key={prop.id} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <TemplateFieldInput
                    propertyId={prop.id}
                    label={prop.name}
                    type={prop.type}
                    value={prop.name}
                    onChange={(val) => updatePropertyName(idx, String(val))}
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)', whiteSpace: 'nowrap' }}>
                    {prop.type}
                  </span>
                </div>
              ))}
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 8 }}>
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
