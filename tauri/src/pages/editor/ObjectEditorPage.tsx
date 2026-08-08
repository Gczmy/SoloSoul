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
import type { PropertyType } from '@/types/template';
import { logger } from '@/lib/logger';
import styles from './ObjectEditorPage.module.css';

// Each template belongs to a workspace section.
// typeId is the section (for filtering), not the template name.
type TemplateCategory = 'identity' | 'travel' | 'financial' | 'professional';

// P050: 字段类型值校验表驱动化。动态组子字段与普通字段共用同一套校验，
// 替代原先两份「switch 六 case」重复逻辑。返回 isValid=false 表示校验失败。
const FIELD_TYPE_VALIDATORS: Partial<
  Record<PropertyType, { hintKey: string; isValid: (value: string) => boolean }>
> = {
  email: {
    hintKey: 'editor:validation_email',
    isValid: (v) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v),
  },
  url: {
    hintKey: 'editor:validation_url',
    // 接受带或不带协议头的 URL；缺失时补 https:// 再验证
    isValid: (v) => {
      const urlStr = /^https?:\/\//i.test(v) ? v : `https://${v}`;
      try {
        new URL(urlStr);
        return true;
      } catch {
        return false;
      }
    },
  },
  phone: {
    hintKey: 'editor:validation_phone',
    isValid: (v) => /^[\d\s\-+()]{3,20}$/.test(v),
  },
  date: {
    hintKey: 'editor:validation_date',
    isValid: (v) => /^\d{4}-\d{2}-\d{2}$/.test(v),
  },
  number: {
    hintKey: 'editor:validation_number',
    isValid: (v) => !Number.isNaN(Number(v)),
  },
};

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
  // P055: 分字段 selector，避免 store 任何变化触发整页重渲染（函数引用稳定）
  const getObject = useObjectStore((s) => s.getObject);
  const createObject = useObjectStore((s) => s.createObject);
  const updateObject = useObjectStore((s) => s.updateObject);
  const currentObjectCache = useObjectStore((s) => s.currentObjectCache);
  const currentObject = objectId ? (currentObjectCache[objectId] ?? null) : null;
  const { onError, onSuccess } = useToastError();
  const userTemplates = useTemplateStore((s) => s.templates);
  const loadUserTemplates = useTemplateStore((s) => s.loadTemplates);
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
    loadUserTemplates().catch((err) => logger.warn('[ObjectEditor] Load templates failed:', err));
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
        allowedTypes?: PropertyType[];
        maxItems?: number;
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
        allowedTypes: p.allowedTypes,
        maxItems: p.maxItems,
      }));
    }
    return map;
  }, [userTemplates]);

  /** Resolve sensitivity level for a property field.
   *  Template default is the single source of truth.
   */
  const getSensitivity = (_fieldKey: string, templateDefault?: string): SensitivityLevel => {
    return (templateDefault as SensitivityLevel) || 'internal';
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
  // 记录已完成加载（无论成败）的对象 ID。替代原先 loadingObjRef + dataLoaded
  // 双状态门控——ref 门控在真机时序下存在死锁窗口（填充 effect bail 后不再触发），
  // 导致页面只剩对象类型一行、字段全部空白（模拟器时序快恰好绕过，故无法复现）。
  const [loadedFor, setLoadedFor] = useState<string | null>(null);
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

  // Determine typeId
  const typeId = isNew
    ? sectionParam || (selectedType ? templateMeta[selectedType]?.category || selectedType : '')
    : currentObject?.typeId || '';

  // Load existing object and populate form
  useEffect(() => {
    if (isNew || !objectId || !accountId) return;
    let cancelled = false;
    // 重置表单状态，避免旧对象的缓存数据被锁定进表单
    setDataLoaded(false);
    setLoadedFor(null);
    setName('');
    setValues({});
    getObject(accountId, objectId)
      .catch((e) => onError(e, t('common:object_load_failed')))
      .finally(() => {
        // 无论成败都标记加载结束：失败已有 toast，表单照常渲染，避免空白页
        if (!cancelled) setLoadedFor(objectId);
      });
    return () => {
      cancelled = true;
    };
  }, [objectId, accountId, getObject, isNew, onError, t]);

  // When currentObject loads (for editing), populate the form
  // Guard: 仅当本次加载已结束（loadedFor === objectId）且缓存对象就是当前对象时才填充，
  // 避免 getObject 解析前用旧缓存（stale cache）提前填充。
  // Template matching re-runs when objectTemplates async-finish loading
  // (fixes "空白内容" bug where templates loaded after dataLoaded was set).
  useEffect(() => {
    if (isNew || loadedFor !== objectId || !currentObject || currentObject.id !== objectId)
      return;

    // Populate property values (only on first load, not on template re-match)
    if (!dataLoaded) {
      setName(currentObject.name || '');
      const vals: Record<string, unknown> = {};
      if (currentObject.properties && typeof currentObject.properties === 'object') {
        const fieldDefs = (currentObject.properties as Record<string, unknown>).__fields as
          | Record<string, { type?: string }>
          | undefined;
        for (const [k, v] of Object.entries(currentObject.properties)) {
          if (k.startsWith('__')) continue;
          const fieldType = fieldDefs?.[k]?.type;
          if (fieldType === 'dynamic_group' && Array.isArray(v)) {
            vals[k] = v;
          } else if (typeof v === 'string') {
            vals[k] = v;
          } else if (typeof v === 'number' || typeof v === 'boolean') {
            vals[k] = String(v);
          } else if (Array.isArray(v)) {
            vals[k] = v;
          } else if (v !== null && v !== undefined) {
            vals[k] = String(v);
          }
        }
      }
      setValues(vals);
    }

    // Detect template (re-runs when objectTemplates changes even after dataLoaded)
    let matchedType = '';
    if (currentObject.templateId) {
      if (objectTemplates[currentObject.templateId]) {
        // 确保模板与对象属于同一页面。若模板被恢复（重新创建）但页面不同，不认为是匹配的模板
        if (templateMeta[currentObject.templateId]?.category === currentObject.typeId) {
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
      const propKeys = Object.keys(values);
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
    } else if (!dataLoaded) {
      // 仅首次加载时清空 selectedType；后续模板加载后不覆盖用户已选
      setSelectedType('');
    }

    if (!dataLoaded) {
      setDataLoaded(true);
    }
  }, [
    currentObject,
    isNew,
    dataLoaded,
    objectId,
    loadedFor,
    objectTemplates,
    templateMeta,
    currentObjectCache,
    values,
  ]);

  const validateFields = (): boolean => {
    const errors: Record<string, string> = {};
    for (const field of fields) {
      const val = values[field.key];

      if (field.type === 'dynamic_group') {
        const items = Array.isArray(val) ? val : [];
        if (field.maxItems !== undefined && items.length > field.maxItems) {
          errors[field.key] = t('editor:dynamic_group_max_items', {
            max: field.maxItems,
          });
          continue;
        }
        const names = new Set<string>();
        for (let i = 0; i < items.length; i++) {
          const item = items[i];
          if (!item || typeof item !== 'object') continue;
          const { name, type, value } = item as Record<string, unknown>;
          if (!name || String(name).trim() === '') {
            errors[field.key] = t('editor:dynamic_group_empty_name', { index: i + 1 });
            break;
          }
          const key = String(name).trim();
          if (names.has(key)) {
            errors[field.key] = t('editor:dynamic_group_duplicate_name', { name: key });
            break;
          }
          names.add(key);
          if (field.allowedTypes?.length && !field.allowedTypes.includes(type as PropertyType)) {
            errors[field.key] = t('editor:dynamic_group_disallowed_type', {
              type,
            });
            break;
          }
          // 按子字段类型做值校验（表驱动，替代原 switch 六 case）
          const strVal = String(value ?? '').trim();
          if (!strVal) continue;
          const validator = FIELD_TYPE_VALIDATORS[type as PropertyType];
          if (validator && !validator.isValid(strVal)) {
            errors[field.key] = t('editor:dynamic_group_invalid_value', {
              index: i + 1,
              hint: t(validator.hintKey),
            });
            break;
          }
        }
        continue;
      }

      const strVal = typeof val === 'string' ? val.trim() : String(val ?? '').trim();

      if (field.required && !strVal) {
        errors[field.key] = t('editor:validation_required', { field: field.label });
        continue;
      }
      if (!strVal) continue;

      // 字段类型值校验（表驱动，替代原 switch 六 case）
      const validator = FIELD_TYPE_VALIDATORS[field.type as PropertyType];
      if (validator && !validator.isValid(strVal)) {
        errors[field.key] = t(validator.hintKey);
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
          typeId,
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
          typeId={typeId}
          currentObject={currentObject}
          contractTypeId={contractTypeId}
          customPages={customPages}
          sectionParam={sectionParam}
        />

        {!isNew && loadedFor !== objectId
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
                <div className={styles.formActions}>
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
