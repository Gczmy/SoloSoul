import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { useToastError } from '@/hooks/useToastError';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { ObjectTemplateSelector } from '@/components/editor/ObjectTemplateSelector';
import { ObjectFieldList } from '@/components/editor/ObjectFieldList';

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
  const { getObject, createObject, updateObject, currentObjectCache } = useObjectStore();
  const currentObject = objectId ? currentObjectCache[objectId] ?? null : null;
  const { onError, onSuccess } = useToastError();
  const { templates: userTemplates, loadTemplates: loadUserTemplates } = useTemplateStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);

  const handleFieldChange = useCallback((key: string, val: unknown) => {
    setValues((v) => ({ ...v, [key]: val }));
  }, []);

  const handleClearError = useCallback((key: string) => {
    setValidationErrors((err) => {
      const next = { ...err };
      delete next[key];
      return next;
    });
  }, []);

  useEffect(() => {
    loadUserTemplates().catch((err) => console.warn('[ObjectEditor] Load templates failed:', err));
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
        contractField?: boolean;
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
        contractField: p.contractField,
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

  // Auto-select template if section provides a clear default (F015: update after
  // templates async-load instead of computing only on first render).
  const [selectedType, setSelectedType] = useState('');
  const hasAutoSelectedRef = useRef(false);
  useEffect(() => {
    if (!hasAutoSelectedRef.current && visibleTemplates.length > 0 && !selectedType) {
      setSelectedType(visibleTemplates[0]);
      hasAutoSelectedRef.current = true;
    }
  }, [visibleTemplates, selectedType, templateMeta]);
  const [name, setName] = useState('');
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [dataLoaded, setDataLoaded] = useState(false);
  const loadingObjRef = useRef(false);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

  const fields = objectTemplates[selectedType] || [];
  const selectedTemplate = userTemplates.find((t) => t.id === selectedType);
  const contractTypeId = currentObject?.contractTypeId || selectedTemplate?.contractTypeId;

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
      loadingObjRef.current = true;
      setDataLoaded(false);
      setName('');
      setValues({});
      getObject(accountId, objectId)
        .catch((e) => onError(e, t('common:object_load_failed')))
        .finally(() => { loadingObjRef.current = false; });
    }
  }, [objectId, accountId, getObject, isNew, onError, t]);

  // When currentObject loads (for editing), populate the form
  // Guard: skip if a fresh fetch is in-flight (prevents stale cache data from
  // populating the form before the most recent getObject resolves).
  useEffect(() => {
    if (isNew || !currentObject || dataLoaded || currentObject.id !== objectId || loadingObjRef.current) return;
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
        // 确保模板与对象属于同一页面。若模板被恢复（重新创建）但页面不同，不认为是匹配的模板
        if (templateMeta[currentObject.templateId]?.category === currentObject.collectionType) {
          matchedType = currentObject.templateId;
        }
      }
      // Backward compat: old user-template objects used utpl_ prefix
      if (!matchedType) {
        const legacyKey = `utpl_${currentObject.templateId}`;
        if (objectTemplates[legacyKey]) {
          matchedType = legacyKey;
        }
      }
    }
    // 仅当对象没有 templateId（旧版对象）时才模糊匹配其他模板。
    // 如果有 templateId 但模板不存在（已删除），不匹配到其他模板，
    // 而是走 __fields 回退路径，避免匹配到同字段 ID 的不同语言模板。
    if (!matchedType && !currentObject.templateId) {
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
    } else {
      // 模板已删除 — 清空 selectedType，触发回退渲染路径
      setSelectedType('');
    }
    setDataLoaded(true);
  }, [currentObject, isNew, dataLoaded, objectId, objectTemplates, templateMeta, currentObjectCache]);

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
          properties: values,
          parentId,
          templateId: selectedType || undefined,
          templateType: selectedType ? 'user' : undefined,
        });
        onSuccess(t('common:object_created'));
      } else {
        await updateObject(objectId!, {
          name: name || t('common:object_name_placeholder'),
          properties: values,
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
      <PageContainer variant="xs" gap="default">
        <ObjectTemplateSelector
          isNew={isNew}
          visibleTemplates={visibleTemplates}
          selectedType={selectedType}
          onSelect={setSelectedType}
          templateMeta={templateMeta}
          userTemplates={userTemplates}
          collectionType={collectionType}
          currentObject={currentObject}
          contractTypeId={contractTypeId}
          customPages={customPages}
          sectionParam={sectionParam}
        />

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
                <ObjectFieldList
                  fields={fields}
                  displayFields={displayFields}
                  values={values}
                  onChange={handleFieldChange}
                  validationErrors={validationErrors}
                  onClearError={handleClearError}
                  currentObject={currentObject}
                  contractTypeId={contractTypeId}
                  getSensitivity={getSensitivity}
                  isNew={isNew}
                />
                <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                  <Button variant="secondary" onClick={() => navigate(-1)}>
                    {t('common:cancel')}
                  </Button>
                  <Button variant="secondary" onClick={handleSave} loading={isSaving}>
                    {t('common:save')}
                  </Button>
                </div>
              </>
            )}
      </PageContainer>
    </AppShell>
  );
}
