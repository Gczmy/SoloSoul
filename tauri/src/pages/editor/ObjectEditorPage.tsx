import { useState, useEffect, useMemo } from 'react';
import { useParams, useNavigate, useSearchParams, useLocation } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { useToastError } from '@/hooks/useToastError';
import { TemplateFieldInput } from '@/components/TemplateFieldInput';
import { LayoutTemplate } from 'lucide-react';
import type { PropertyType } from '@/types/template';
import { useTemplateStore } from '@/stores/templateStore';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';

// Each template belongs to a workspace section.
// collectionType is the section (for filtering), not the template name.
type TemplateCategory = 'identity' | 'travel' | 'financial' | 'professional';

export function ObjectEditorPage() {
  const { objectId } = useParams();
  const navigate = useNavigate();
  const location = useLocation();
  const [searchParams] = useSearchParams();
  // Read section and parentId from URL params
  const sectionParam = searchParams.get('section') || '';
  const parentId = searchParams.get('parentId') || undefined;

  const isNew = !objectId;
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'editor', 'navigation']);
  const { getObject, createObject, updateObject, currentObject } = useObjectStore();
  const { onError, onSuccess } = useToastError();
  const { templates: userTemplates, loadTemplates: loadUserTemplates } = useTemplateStore();

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
    const map: Record<
      string,
      {
        key: string;
        label: string;
        type: string;
        options?: string[];
        sensitivityLevel?: string;
        required?: boolean;
        deprecatedAt?: string;
      }[]
    > = {};
    for (const tpl of userTemplates) {
      map[tpl.id] = tpl.properties.map((p) => ({
        key: p.id,
        label: p.name,
        type: p.type,
        options: p.options,
        sensitivityLevel: p.sensitivityLevel || 'internal',
        required: false,
        deprecatedAt: p.deprecatedAt,
      }));
    }
    return map;
  }, [userTemplates]);

  /** Resolve sensitivity level for a property field.
   *  Template default is the single source of truth.
   */
  const getSensitivity = (_fieldKey: string, templateDefault?: string): SensitivityLevel => {
    return (templateDefault as SensitivityLevel) || 'public';
  };

  // Filter templates to only show those belonging to the current section/page
  const visibleTemplates = useMemo(() => {
    if (sectionParam) {
      return Object.keys(objectTemplates).filter((t) => templateMeta[t]?.category === sectionParam);
    }
    if (parentId) {
      return Object.keys(objectTemplates).filter((t) => templateMeta[t]?.category === parentId);
    }
    return Object.keys(objectTemplates);
  }, [sectionParam, parentId, objectTemplates, templateMeta]);

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

  const activeFields = fields.filter((f) => !f.deprecatedAt);
  const deprecatedFields = isNew
    ? []
    : fields.filter(
        (f) =>
          f.deprecatedAt &&
          values[f.key] !== undefined &&
          values[f.key] !== '' &&
          values[f.key] !== null,
      );
  const displayFields = [...activeFields, ...deprecatedFields];

  // Determine collectionType
  const collectionType = isNew
    ? sectionParam || (selectedType ? templateMeta[selectedType]?.category || selectedType : '')
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
    const vals: Record<string, unknown> = {};
    if (currentObject.properties && typeof currentObject.properties === 'object') {
      for (const [k, v] of Object.entries(currentObject.properties)) {
        if (typeof v === 'string') vals[k] = v;
        else if (typeof v === 'number' || typeof v === 'boolean') vals[k] = String(v);
        else if (Array.isArray(v)) vals[k] = v;
        else if (v !== null && v !== undefined) vals[k] = String(v);
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
          // Accept URLs with or without protocol; prepend https:// if missing
          const urlStr = /^https?:\/\//i.test(strVal) ? strVal : `https://${strVal}`;
          try {
            new URL(urlStr);
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
      onError(t('editor:validation_failed'), t('common:object_save_failed'));
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

  const handleBack = () => {
    // TemplateManager already returns to /editor with replace: true,
    // so a single history step back correctly lands on the previous page (workspace).
    navigate(-1);
  };

  return (
    <AppShell title={isNew ? t('common:new_object') : t('common:edit_object')} onBack={handleBack}>
      <div
        style={{
          maxWidth: 560,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        {isNew && (
          <Card>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
              {t('common:object_type')}
              {sectionParam && (
                <span
                  style={{
                    fontSize: 11,
                    color: 'var(--text-tertiary)',
                    marginLeft: 8,
                    fontWeight: 400,
                  }}
                >
                  {t('editor:in_section', {
                    section: t(`navigation:${sectionParam}`, sectionParam),
                  })}
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
                      padding: '10px 16px',
                      borderRadius: 8,
                      border: '1px solid var(--border-subtle)',
                      background:
                        selectedType === type ? 'var(--accent-primary)' : 'var(--bg-elevated)',
                      color: selectedType === type ? 'white' : 'var(--text-primary)',
                      fontSize: 13,
                      cursor: 'pointer',
                    }}
                  >
                    {label}
                  </button>
                );
              })}
              <button
                onClick={() =>
                  navigate('/settings/templates', {
                    state: { from: location.pathname + location.search },
                  })
                }
                style={{
                  marginLeft: 'auto',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  padding: '8px 12px',
                  borderRadius: 8,
                  border: '1px dashed var(--border-strong)',
                  background: 'transparent',
                  color: 'var(--text-secondary)',
                  fontSize: 12,
                  cursor: 'pointer',
                }}
                title={t('editor:manage_templates')}
              >
                <LayoutTemplate size={14} /> {t('editor:manage_templates')}
              </button>
              {visibleTemplates.length === 0 && (
                <div style={{ fontSize: 13, color: 'var(--text-tertiary)', padding: '8px 0' }}>
                  {t('editor:no_template_for_section') || '此页面暂无模板，'}
                  <span
                    onClick={() =>
                      navigate('/settings/templates', {
                        state: { from: location.pathname + location.search },
                      })
                    }
                    style={{
                      color: 'var(--accent-primary)',
                      cursor: 'pointer',
                      textDecoration: 'underline',
                    }}
                  >
                    {t('editor:go_create_template') || '前往模板管理新建'}
                  </span>
                </div>
              )}
            </div>
          </Card>
        )}

        {!isNew && collectionType && (
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                {t('common:object_type')}:
              </span>
              <span
                style={{
                  fontSize: 12,
                  fontWeight: 500,
                  padding: '2px 8px',
                  borderRadius: 4,
                  background: 'rgba(91,124,153,0.08)',
                  color: 'var(--accent-primary)',
                }}
              >
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

        {!isNew && !dataLoaded
          ? null
          : (selectedType || !isNew) && (
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
                    {fields.length === 0
                      ? // Fallback: render raw properties as generic text inputs when no template matches
                        Object.entries(values)
                          .filter(([k]) => !k.startsWith('__'))
                          .map(([key, val]) => {
                            const tplField = userTemplates
                              .find((t) => t.id === selectedType)
                              ?.properties.find((p) => p.id === key);
                            const isDeprecated = !!tplField?.deprecatedAt;
                            return (
                              <div key={key}>
                                <Input
                                  label={t(`editor:fields.${key}`, key)}
                                  icon={<FieldTypeIcon type="text" />}
                                  value={String(val ?? '')}
                                  onChange={(e) =>
                                    setValues((v) => ({ ...v, [key]: e.target.value }))
                                  }
                                  placeholder={key}
                                  badge={isDeprecated ? <DeprecatedBadge /> : undefined}
                                />
                              </div>
                            );
                          })
                      : displayFields.map((field) => {
                          const sensitivity = getSensitivity(field.key, field.sensitivityLevel);
                          const fieldLabel = t(`editor:fields.${field.key}`, field.label);
                          // Map legacy frontend type names to PropertyType
                          const propType: PropertyType =
                            field.type === 'tel'
                              ? 'phone'
                              : field.type === 'datetime-local'
                                ? 'datetime'
                                : (field.type as PropertyType) || 'text';
                          const isDeprecated = !!field.deprecatedAt;
                          return (
                            <div key={field.key}>
                              <TemplateFieldInput
                                propertyId={field.key}
                                label={fieldLabel}
                                type={propType}
                                options={field.options}
                                value={values[field.key]}
                                icon={<FieldTypeIcon type={propType} />}
                                badge={
                                  <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                                    <SensitivityBadge level={sensitivity} />
                                    {isDeprecated && <DeprecatedBadge />}
                                  </div>
                                }
                                hint={
                                  ['email', 'url', 'phone', 'date', 'number'].includes(propType)
                                    ? t(`editor:validation_hint_${propType}`)
                                    : undefined
                                }
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
                        })}
                  </div>
                </Card>
                <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                  <Button variant="secondary" onClick={() => navigate(-1)}>
                    {t('common:cancel')}
                  </Button>
                  <Button onClick={handleSave} loading={isSaving}>
                    {t('common:save')}
                  </Button>
                </div>
              </>
            )}
      </div>
    </AppShell>
  );
}
