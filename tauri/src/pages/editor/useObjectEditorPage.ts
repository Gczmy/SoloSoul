import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore, type ObjectData } from '@/stores/objectStore';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { useToastError } from '@/hooks/useToastError';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';
import type { PropertyType, UserTemplate } from '@/types/template';
import type { FieldSuggestion } from '@/components/editor/FieldSuggestions';
import { invokeTypedCommand } from '@/lib/typedIpc';
import { resolveCanonicalFieldName } from '@/lib/fieldNameAliases';
import { resolveFieldSensitivity } from '@/lib/fieldSensitivity';
import { logger } from '@/lib/logger';
import { backendErrorLogDetails } from '@/lib/backendErrorWire';
import { FIELD_TYPE_VALIDATORS } from '@/lib/fieldValidators';
import { SYSTEM_PAGE_KEYS } from '@/components/layout/useNavigationItems';
import { createSessionRequests } from '@/lib/sessionRequests';

interface FieldDef {
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
}

export interface UseObjectEditorPageResult {
  isNew: boolean;
  objectId: string | undefined;
  isObjectReady: boolean;
  isLoadingObject: boolean;
  objectLoadFailed: boolean;
  retryObjectLoad: () => void;
  visibleTemplates: string[];
  selectedType: string;
  setSelectedType: React.Dispatch<React.SetStateAction<string>>;
  templateMeta: Record<string, { category: string; label: string }>;
  userTemplates: UserTemplate[];
  typeId: string;
  currentObject: ObjectData | null;
  contractTypeId: string | undefined;
  customPages: CustomPage[];
  name: string;
  setName: React.Dispatch<React.SetStateAction<string>>;
  fields: FieldDef[];
  displayFields: FieldDef[];
  values: Record<string, unknown>;
  handleFieldChange: (key: string, val: unknown) => void;
  validationErrors: Record<string, string>;
  handleClearError: (key: string) => void;
  getSensitivity: (fieldKey: string, templateDefault?: string) => SensitivityLevel;
  /** 其他对象同名字段的推荐内容（按字段名分组；字段名 → 条目列表）。 */
  fieldSuggestions: Record<string, FieldSuggestion[]>;
  handleSave: () => Promise<void>;
  handleBack: () => void;
  isSaving: boolean;
}

/** 先构造完整字段值，避免字段树先挂载空值再逐项回填。 */
function objectFormValues(object: ObjectData): Record<string, unknown> {
  const values: Record<string, unknown> = {};
  const definitions = object.properties.__fields as Record<string, { type?: string }> | undefined;
  for (const [key, value] of Object.entries(object.properties)) {
    if (key.startsWith('__')) continue;
    if (definitions?.[key]?.type === 'dynamic_group' && Array.isArray(value)) values[key] = value;
    else if (typeof value === 'string' || Array.isArray(value)) values[key] = value;
    else if (value !== null && value !== undefined) values[key] = String(value);
  }
  return values;
}

function matchObjectTemplate(object: ObjectData, templates: UserTemplate[]): string {
  if (object.templateId) {
    const exact = templates.find(
      (template) =>
        template.id === object.templateId && (template.category || 'identity') === object.typeId,
    );
    if (exact) return exact.id;
    // 保留旧版 utpl_ 前缀兼容；明确指定但已删除的模板仍走 __fields 回退。
    return templates.find((template) => template.id === `utpl_${object.templateId}`)?.id ?? '';
  }
  const keys = Object.keys(objectFormValues(object));
  let bestScore = 0;
  let matched = '';
  for (const template of templates) {
    const matches = template.properties.filter((field) => keys.includes(field.id)).length;
    if (matches > bestScore && matches >= template.properties.length * 0.5) {
      bestScore = matches;
      matched = template.id;
    }
  }
  return matched;
}

/**
 * 对象新建/编辑页的全部状态与逻辑：模板加载与匹配（含旧版 utpl_ 兼容与
 * __fields 回退）、完整快照切换与请求隔离、动态组与字段类型校验
 * （表驱动）、新建/更新保存编排。
 */
export function useObjectEditorPage(): UseObjectEditorPageResult {
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
  // 只发布已完整装配的对象快照。路由切换期间保留上一帧，但禁止编辑/保存。
  const [currentObject, setCurrentObject] = useState<ObjectData | null>(null);
  const [requests] = useState(createSessionRequests);
  useEffect(() => () => requests.invalidate(), [requests]);
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

  const templatesReady = useRef<Promise<void>>(Promise.resolve());
  useEffect(() => {
    templatesReady.current = loadUserTemplates().catch((err) =>
      logger.warn('[ObjectEditor] Load templates failed:', err),
    );
  }, [loadUserTemplates]);

  // Build templateMeta and objectTemplates from loaded user templates
  const templateMeta = useMemo(() => {
    const meta: Record<string, { category: string; label: string }> = {};
    for (const tpl of userTemplates) {
      meta[tpl.id] = {
        category: tpl.category || 'identity',
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
  const getSensitivity = (fieldKey: string, templateDefault?: string): SensitivityLevel => {
    return resolveFieldSensitivity({
      fieldId: fieldKey,
      template: { sensitivityLevel: templateDefault },
    });
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
    if (isNew && !hasAutoSelectedRef.current && visibleTemplates.length > 0 && !selectedType) {
      setSelectedType(visibleTemplates[0]);
      hasAutoSelectedRef.current = true;
    }
  }, [isNew, visibleTemplates, selectedType, templateMeta]);
  const [name, setName] = useState('');
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [loadedFor, setLoadedFor] = useState<string | null>(null);
  const [failedFor, setFailedFor] = useState<string | null>(null);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const isObjectReady = isNew || (loadedFor === objectId && currentObject?.id === objectId);
  const objectLoadFailed = !isNew && failedFor === objectId;
  const isLoadingObject = !isObjectReady && !objectLoadFailed;
  const retryObjectLoad = () => setLoadAttempt((attempt) => attempt + 1);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  // 字段推荐：其他对象中「同名字段且有内容」的标量值，按字段名分组供编辑器展示。
  // 一次加载全账户（排除当前编辑对象），失败仅降级为无推荐（不阻塞编辑流程）。
  const [fieldSuggestions, setFieldSuggestions] = useState<Record<string, FieldSuggestion[]>>({});
  const [suggestionsFor, setSuggestionsFor] = useState<string | null>(null);
  const suggestionObjectId = isNew ? null : (currentObject?.id ?? null);

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
    ? sectionParam ||
      parentId ||
      (selectedType ? templateMeta[selectedType]?.category || selectedType : '')
    : currentObject?.typeId || '';
  // 全局入口按模板分类推导自定义父页面，兼容桌面及已有的无参数编辑器入口。
  const destinationPage = customPages.find((page) => page.id === typeId && !page.deletedAt);
  const resolvedParentId = parentId || destinationPage?.id;

  // 推荐跟随已显示的完整快照，等待新对象时不清空上一帧，也不混入另一对象的推荐。
  useEffect(() => {
    if (!accountId || (!isNew && !suggestionObjectId)) return;
    let cancelled = false;
    invokeTypedCommand('object_field_suggestions', {
      accountId,
      excludeObjectId: suggestionObjectId,
    })
      .then((items) => {
        if (cancelled) return;
        const grouped: Record<string, FieldSuggestion[]> = {};
        for (const item of items) {
          // 跨语言归一：中文名/英文名等同一字段的本地化名归到同一规范键
          // （如「出生日期」与「Date of Birth」→ dateOfBirth）
          const canonical = resolveCanonicalFieldName(item.fieldKey, item.fieldName);
          (grouped[canonical] ||= []).push(item);
        }
        setFieldSuggestions(grouped);
        setSuggestionsFor(suggestionObjectId);
      })
      .catch((err) =>
        logger.warn('[ObjectEditor] Load field suggestions failed:', backendErrorLogDetails(err)),
      );
    return () => {
      cancelled = true;
    };
  }, [accountId, isNew, suggestionObjectId]);

  // 同一路由组件保留页面框架。等对象和首轮模板读取结束，一次提交名称、字段、
  // 模板及校验状态；缓存更新本身不能提前放开保存，也不能展示半填充的表单。
  useEffect(() => {
    if (isNew || !objectId || !accountId) return;
    const request = requests.begin('editor-load', accountId);
    requests.invalidate('editor-save');
    setIsSaving(false);
    setFailedFor(null);
    setLoadedFor(null);
    Promise.all([getObject(accountId, objectId), templatesReady.current])
      .then(([object]) => {
        if (!request.isCurrent()) return;
        if (!object || object.id !== objectId || object.accountId !== accountId) {
          setFailedFor(objectId);
          return;
        }
        setCurrentObject(object);
        setName(object.name || '');
        setValues(objectFormValues(object));
        setSelectedType(matchObjectTemplate(object, useTemplateStore.getState().templates));
        setValidationErrors({});
        setLoadedFor(objectId);
      })
      .catch((error) => {
        if (!request.isCurrent()) return;
        setFailedFor(objectId);
        onError(error, t('common:object_load_failed'));
      });
    return () => {
      requests.invalidate('editor-load');
      requests.invalidate('editor-save');
    };
  }, [objectId, accountId, getObject, isNew, onError, t, requests, loadAttempt]);

  // 后续模板刷新只重新匹配字段定义，不覆盖当前草稿和校验结果。
  useEffect(() => {
    if (!isNew && currentObject) {
      setSelectedType(matchObjectTemplate(currentObject, userTemplates));
    }
  }, [currentObject, isNew, userTemplates]);

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
    if (!accountId || !isObjectReady || isSaving) return;
    if (isNew) {
      const knownDestination = SYSTEM_PAGE_KEYS.some((key) => key === typeId) || destinationPage;
      if (
        !knownDestination ||
        (parentId && parentId !== destinationPage?.id) ||
        !selectedType ||
        !visibleTemplates.includes(selectedType) ||
        templateMeta[selectedType]?.category !== typeId
      ) {
        onError(t('editor:destination_unavailable'), t('common:object_save_failed'));
        return;
      }
    }
    if (!validateFields()) {
      onError(t('editor:validation_failed'), t('common:object_save_failed'));
      return;
    }
    const request = requests.begin('editor-save', accountId);
    setIsSaving(true);
    try {
      if (isNew) {
        await createObject({
          accountId,
          name: name || templateMeta[selectedType]?.label || 'Untitled',
          typeId,
          properties: values,
          parentId: resolvedParentId,
          templateId: selectedType || undefined,
          templateType: selectedType ? 'user' : undefined,
        });
        if (!request.isCurrent()) return;
        onSuccess(t('common:object_created'));
      } else {
        await updateObject(objectId!, {
          name: name || t('common:object_name_placeholder'),
          properties: values,
        });
        if (!request.isCurrent()) return;
        onSuccess(t('common:object_saved'));
      }
      navigate(-1);
    } catch (e) {
      if (request.isCurrent()) onError(e, t('common:object_save_failed'));
    } finally {
      if (request.isCurrent()) setIsSaving(false);
    }
  };

  const handleBack = () => {
    // TemplateManager already returns to /editor with replace: true,
    // so a single history step back correctly lands on the previous page (workspace).
    navigate(-1);
  };

  return {
    isNew,
    objectId,
    isObjectReady,
    isLoadingObject,
    objectLoadFailed,
    retryObjectLoad,
    visibleTemplates,
    selectedType,
    setSelectedType,
    templateMeta,
    userTemplates,
    typeId,
    currentObject,
    contractTypeId,
    customPages,
    name,
    setName,
    fields,
    displayFields,
    values,
    handleFieldChange,
    validationErrors,
    handleClearError,
    getSensitivity,
    fieldSuggestions: suggestionsFor === suggestionObjectId ? fieldSuggestions : {},
    handleSave,
    handleBack,
    isSaving,
  };
}
