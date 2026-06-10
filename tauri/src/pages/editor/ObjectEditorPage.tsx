import { useState, useEffect, useMemo } from 'react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSensitivityStore, SensitivityLevel } from '@/stores/sensitivityStore';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { useToastError } from '@/hooks/useToastError';
import { TemplateFieldInput } from '@/components/TemplateFieldInput';
import type { PropertyType } from '@/types/template';
import { useTemplateStore } from '@/stores/templateStore';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';

// Each template belongs to a workspace section.
// collectionType is the section (for filtering), not the template name.
type TemplateCategory = 'identity' | 'travel' | 'financial' | 'professional';

export function ObjectEditorPage() {
  const { objectId } = useParams();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  // Read section and parentId from URL params
  const sectionParam = searchParams.get('section') || '';
  const parentId = searchParams.get('parentId') || undefined;

  const isNew = !objectId;
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'editor', 'navigation']);
  const { getObject, createObject, updateObject, currentObject } = useObjectStore();
  const { map: sensitivityMap, loadMap } = useSensitivityStore();
  const { onError, onSuccess } = useToastError();
  const { templates: userTemplates, loadTemplates: loadUserTemplates } = useTemplateStore();

  // Load user templates and sensitivity map on mount
  useEffect(() => { loadMap(); }, []);
  useEffect(() => {
    loadUserTemplates().catch(() => {});
  }, [loadUserTemplates]);

  // Build templateMeta and objectTemplates from loaded user templates
  const templateMeta = useMemo(() => {
    const meta: Record<string, { category: TemplateCategory; label: string }> = {};
    for (const tpl of userTemplates) {
      meta[tpl.id] = {
        category: (tpl.category || 'identity') as TemplateCategory,
        label: tpl.name,
      };
    }
    return meta;
  }, [userTemplates]);

  const objectTemplates = useMemo(() => {
    const map: Record<string, { key: string; label: string; type: string; sensitivityLevel?: string; required?: boolean }[]> = {};
    for (const tpl of userTemplates) {
      map[tpl.id] = tpl.properties.map((p) => ({
        key: p.id,
        label: p.name,
        type: p.type,
        sensitivityLevel: p.sensitivityLevel || 'internal',
        required: false,
      }));
    }
    return map;
  }, [userTemplates]);

  /** Resolve sensitivity level for a property field.
   *  Priority: sensitivityMap (user override) > template default > 'public' fallback.
   */
  const getSensitivity = (fieldKey: string, templateDefault?: string): SensitivityLevel => {
    const ct = collectionType || sectionParam || '';
    const fieldId = `${ct}.${fieldKey}`;
    if (sensitivityMap?.entries?.[fieldId]) return sensitivityMap.entries[fieldId];
    // Try snake_case normalization (map entries use snake_case, fields use camelCase)
    const snakeKey = fieldKey.replace(/[A-Z]/g, (c) => '_' + c.toLowerCase());
    const snakeFieldId = `${ct}.${snakeKey}`;
    if (snakeFieldId !== fieldId && sensitivityMap?.entries?.[snakeFieldId]) return sensitivityMap.entries[snakeFieldId];
    // Fallback: match any entry ending with .{key}
    for (const [id, level] of Object.entries(sensitivityMap?.entries || {})) {
      if (id.endsWith(`.${fieldKey}`) || id.endsWith(`.${snakeKey}`)) return level;
    }
    return (templateDefault as SensitivityLevel) || 'public';
  };

  // Filter templates to only show those belonging to the current section
  const visibleTemplates = useMemo(() => {
    if (!sectionParam) return Object.keys(objectTemplates);
    return Object.keys(objectTemplates).filter(
      (t) => templateMeta[t]?.category === sectionParam
    );
  }, [sectionParam]);

  // Auto-select template if section provides a clear default
  const [selectedType, setSelectedType] = useState(() => {
    if (sectionParam && visibleTemplates.length > 0) {
      return visibleTemplates[0];
    }
    return '';
  });
  const [name, setName] = useState('');
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [dataLoaded, setDataLoaded] = useState(false);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

  const fields = objectTemplates[selectedType] || [];

  // Determine collectionType
  const collectionType = isNew
    ? sectionParam || (selectedType ? (templateMeta[selectedType]?.category || selectedType) : '')
    : currentObject?.collectionType || '';

  // Load existing object and populate form
  useEffect(() => {
    if (!isNew && objectId && accountId) {
      // Reset form state before fetching so stale store data doesn't get locked in
      setDataLoaded(false);
      setName('');
      setValues({});
      getObject(accountId, objectId).catch((e) => onError(e, t('common:object_load_failed')));
    }
  }, [objectId, accountId]);

  // When currentObject loads (for editing), populate the form
  useEffect(() => {
    if (isNew || !currentObject || dataLoaded || currentObject.id !== objectId) return;
    setName(currentObject.name || '');
    // Populate property values
    const vals: Record<string, string> = {};
    if (currentObject.properties && typeof currentObject.properties === 'object') {
      for (const [k, v] of Object.entries(currentObject.properties)) {
        if (typeof v === 'string') vals[k] = v;
        else if (typeof v === 'number' || typeof v === 'boolean') vals[k] = String(v);
        else if (v !== null && v !== undefined) vals[k] = JSON.stringify(v);
      }
    }
    setValues(vals);
    // Detect template from stored templateId first, then fall back to property keys
    let matchedType = '';
    if (currentObject.templateId) {
      if (objectTemplates[currentObject.templateId]) {
        matchedType = currentObject.templateId;
      }
      // Backward compat: old user-template objects used utpl_ prefix
      if (!matchedType) {
        const legacyKey = `utpl_${currentObject.templateId}`;
        if (objectTemplates[legacyKey]) {
          matchedType = legacyKey;
        }
      }
    }
    if (!matchedType) {
      const propKeys = Object.keys(vals);
      let bestScore = 0;
      for (const [tplName, tplFields] of Object.entries(objectTemplates)) {
        const tplKeys = tplFields.map((f) => f.key);
        const matchCount = tplKeys.filter((k) => propKeys.includes(k)).length;
        if (matchCount > bestScore && matchCount >= tplKeys.length * 0.5) {
          bestScore = matchCount;
          matchedType = tplName;
        }
      }
    }
    if (matchedType) {
      setSelectedType(matchedType);
    }
    setDataLoaded(true);
  }, [currentObject, isNew, dataLoaded]);

  const validateFields = (): boolean => {
    const errors: Record<string, string> = {};
    for (const field of fields) {
      const val = values[field.key];
      const strVal = typeof val === 'string' ? val.trim() : String(val ?? '').trim();

      if (field.required && !strVal) {
        errors[field.key] = t('editor:validation_required', { field: field.label });
        continue;
      }
      if (!strVal) continue;

      switch (field.type) {
        case 'email': {
          const emailRe = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
          if (!emailRe.test(strVal)) {
            errors[field.key] = t('editor:validation_email');
          }
          break;
        }
        case 'url': {
          try {
            new URL(strVal);
          } catch {
            errors[field.key] = t('editor:validation_url');
          }
          break;
        }
        case 'phone': {
          const phoneRe = /^[\d\s\-+()]{3,20}$/;
          if (!phoneRe.test(strVal)) {
            errors[field.key] = t('editor:validation_phone');
          }
          break;
        }
        case 'date': {
          const dateRe = /^\d{4}-\d{2}-\d{2}$/;
          if (!dateRe.test(strVal)) {
            errors[field.key] = t('editor:validation_date');
          }
          break;
        }
        case 'number': {
          if (Number.isNaN(Number(strVal))) {
            errors[field.key] = t('editor:validation_number');
          }
          break;
        }
      }
    }
    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSave = async () => {
    if (!accountId) return;
    if (!validateFields()) {
      onError(t('editor:validation_failed'), t('editor:validation_failed'));
      return;
    }
    setIsSaving(true);
    try {
      if (isNew) {
        await createObject({
          accountId,
          name: name || templateMeta[selectedType]?.label || 'Untitled',
          collectionType,
          properties: values as unknown as Record<string, unknown>,
          parentId,
          templateId: selectedType || undefined,
          templateType: selectedType ? 'user' : undefined,
        });
        onSuccess(t('common:object_created'));
      } else {
        await updateObject(objectId!, {
          name: name || t('common:object_name_placeholder'),
          properties: values as unknown as Record<string, unknown>,
        });
        onSuccess(t('common:object_saved'));
      }
      navigate(-1);
    } catch (e) {
      onError(e, t('common:object_save_failed'));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <AppShell title={isNew ? t('common:new_object') : t('common:edit_object')} onBack={() => navigate(-1)}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {isNew && (
          <Card>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
              {t('common:object_type')}
              {sectionParam && (
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 8, fontWeight: 400 }}>
                  {t('editor:in_section', { section: t(`navigation:${sectionParam}`, sectionParam) })}
                </span>
              )}
            </h3>
            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
              {visibleTemplates.map((type) => {
                const label = templateMeta[type]?.label || type;
                return (
                  <button
                    key={type}
                    onClick={() => setSelectedType(type)}
                    style={{
                      padding: '10px 16px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                      background: selectedType === type ? 'var(--accent-primary)' : 'var(--bg-elevated)',
                      color: selectedType === type ? 'white' : 'var(--text-primary)',
                      fontSize: 13, cursor: 'pointer',
                    }}
                  >
                    {label}
                  </button>
                );
              })}
            </div>
          </Card>
        )}

        {!isNew && collectionType && (
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{t('common:object_type')}:</span>
              <span style={{
                fontSize: 12, fontWeight: 500, padding: '2px 8px', borderRadius: 4,
                background: 'rgba(91,124,153,0.08)', color: 'var(--accent-primary)',
              }}>
                {t(`navigation:${collectionType}`, collectionType)}
              </span>
              {selectedType && (
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                  · {templateMeta[selectedType]?.label || selectedType}
                </span>
              )}
            </div>
          </Card>
        )}

        {(!isNew && !dataLoaded) ? null : (selectedType || !isNew) && (
          <>
            <Card>
              <Input
                label={t('common:object_name')}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t('common:object_name_placeholder')}
              />
            </Card>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
                {t('common:properties')}
              </h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {fields.length === 0 ? (
                  // Fallback: render raw properties as generic text inputs when no template matches
                  Object.entries(values).filter(([k]) => !k.startsWith('__')).map(([key, val]) => (
                    <div key={key}>
                      <Input
                        label={t(`editor:fields.${key}`, key)}
                        icon={<FieldTypeIcon type="text" />}
                        value={String(val ?? '')}
                        onChange={(e) => setValues((v) => ({ ...v, [key]: e.target.value }))}
                        placeholder={key}
                      />
                    </div>
                  ))
                ) : (
                fields.map((field) => {
                    const sensitivity = getSensitivity(field.key, field.sensitivityLevel);
                    const fieldLabel = t(`editor:fields.${field.key}`, field.label);
                    // Map legacy frontend type names to PropertyType
                    const propType: PropertyType =
                      field.type === 'tel' ? 'phone' :
                      field.type === 'datetime-local' ? 'datetime' :
                      (field.type as PropertyType) || 'text';
                    return (
                  <div key={field.key}>
                    <TemplateFieldInput
                      propertyId={field.key}
                      label={fieldLabel}
                      type={propType}
                      value={values[field.key]}
                      icon={<FieldTypeIcon type={propType} />}
                      badge={<SensitivityBadge level={sensitivity} />}
                      onChange={(val) => {
                        setValues((v) => ({ ...v, [field.key]: val }));
                        if (validationErrors[field.key]) {
                          setValidationErrors((err) => {
                            const next = { ...err };
                            delete next[field.key];
                            return next;
                          });
                        }
                      }}
                    />
                    {validationErrors[field.key] && (
                      <div style={{ fontSize: 11, color: '#ef4444', marginTop: 4 }}>
                        {validationErrors[field.key]}
                      </div>
                    )}
                  </div>
                    );
                  })
                )}
              </div>
            </Card>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              {!isNew && objectId && (
                <>
                <Button variant="secondary" onClick={async () => {
                  const name = prompt(t('common:template_name_prompt'), currentObject?.name || '');
                  if (name && objectId) {
                    try {
                      const newTemplateId = await invoke<string>('template_save_from_object', {
                        objectId,
                        templateName: name,
                        iconId: undefined,
                      });
                      alert(t('common:template_saved') + ' (ID: ' + newTemplateId.slice(0, 12) + '...)');
                    } catch (e) { alert(t('common:template_save_failed') + ': ' + e); }
                  }
                }}>
                  {t('common:save_as_template')}
                </Button>
                <Button variant="secondary" onClick={async () => {
                  const path = await open({ multiple: false, title: t('common:select_file_attach') });
                  if (path && typeof path === 'string' && objectId) {
                    try {
                      await invoke('attachment_save', { objectId, meta: {
                        id: crypto.randomUUID(), objectId,
                        fileName: path.split('/').pop() || 'file',
                        mimeType: 'application/octet-stream', sizeBytes: 0, createdAt: new Date().toISOString(),
                      }});
                      alert(t('common:attachment_added'));
                    } catch (e) { alert(t('common:attachment_failed') + ': ' + e); }
                  }
                }}>
                  {t('common:add_attachment')}
                </Button>
                </>
              )}
              <Button variant="secondary" onClick={() => navigate(-1)}>{t('common:cancel')}</Button>
              <Button onClick={handleSave} loading={isSaving}>{t('common:save')}</Button>
            </div>
          </>
        )}
      </div>
    </AppShell>
  );
}
