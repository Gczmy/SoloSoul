import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useTemplateStore } from '@/stores/templateStore';
import { useUiStore } from '@/stores/uiStore';
import { usePluginStore } from '@/stores/pluginStore';
import { useConfirm } from '@/hooks/useConfirm';
import { deriveContractBindings, type PluginManifest } from '@/lib/plugin';
import type {
  UserTemplate,
  TemplateProperty,
  PropertyType,
  SensitivityLevel,
  ContractRoleBinding,
} from '@/types/template';

const EMPTY_PLUGINS: PluginManifest[] = [];

/**
 * 模板编辑器状态与操作（P224-③ 拆分，自 TemplateManagerPage 收敛）。
 * 承载编辑器全部本地状态（字段/动态组/废弃字段/命名校验）与保存/移除/动态组回调，
 * 数据经 TemplateManagerPage 透传（模板 store 选择器在此复用）。
 */
export function useTemplateEditor() {
  const { t } = useTranslation(['common', 'settings', 'editor']);
  const showToast = useUiStore((s) => s.showToast);
  const checkFieldUsage = useTemplateStore((s) => s.checkFieldUsage);
  const createTemplate = useTemplateStore((s) => s.createTemplate);
  const updateTemplate = useTemplateStore((s) => s.updateTemplate);
  const loadTemplates = useTemplateStore((s) => s.loadTemplates);
  const installedPlugins = usePluginStore((s) => s.installedPlugins) ?? EMPTY_PLUGINS;
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  const [editingTemplate, setEditingTemplate] = useState<UserTemplate | null>(null);
  const [isNewTemplate, setIsNewTemplate] = useState(false);
  const [editName, setEditName] = useState('');
  const [editCategory, setEditCategory] = useState<string>('identity');
  const [editIconId, setEditIconId] = useState<string>('document');
  const [editContractTypeId, setEditContractTypeId] = useState<string>('');
  const [editProperties, setEditProperties] = useState<TemplateProperty[]>([]);

  const [newFieldType, setNewFieldType] = useState<PropertyType>('text');

  // 动态字段组（模板级开关）
  const [dynamicGroupEnabled, setDynamicGroupEnabled] = useState(false);
  const [dynamicGroupAllowedTypes, setDynamicGroupAllowedTypes] = useState<
    PropertyType[] | undefined
  >();
  const [dynamicGroupMaxItems, setDynamicGroupMaxItems] = useState<number | undefined>();
  const [dynamicGroupSensitivity, setDynamicGroupSensitivity] =
    useState<SensitivityLevel>('internal');
  const [showDeprecated, setShowDeprecated] = useState(false);
  const [fieldUsageMap, setFieldUsageMap] = useState<
    Record<string, { active: number; softDeleted: number }>
  >({});
  const [showNameError, setShowNameError] = useState(false);

  // Load field usage for deprecated fields
  const loadFieldUsage = useCallback(async () => {
    if (!editingTemplate) {
      setFieldUsageMap({});
      return;
    }
    const deprecated = editProperties.filter((p) => p.deprecatedAt);
    if (deprecated.length === 0) return;
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
  }, [editingTemplate, editProperties, checkFieldUsage]);

  useEffect(() => {
    loadFieldUsage();
  }, [loadFieldUsage]);

  const openEdit = (tpl: UserTemplate) => {
    setIsNewTemplate(false);
    setEditingTemplate(tpl);
    setEditName(tpl.name);
    setEditCategory(tpl.category || 'identity');
    setEditIconId(tpl.iconId || 'document');
    setEditContractTypeId(tpl.contractTypeId || '');
    setEditProperties([...tpl.properties]);

    // 初始化动态字段组状态
    const dg = tpl.properties.find((p) => p.type === 'dynamic_group');
    setDynamicGroupEnabled(!!dg);
    setDynamicGroupAllowedTypes(dg?.allowedTypes);
    setDynamicGroupMaxItems(dg?.maxItems);
    setDynamicGroupSensitivity(dg?.sensitivityLevel || 'internal');
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
    setEditIconId('document');
    setEditContractTypeId('');
    setEditProperties([]);
    setDynamicGroupEnabled(false);
    setDynamicGroupAllowedTypes(undefined);
    setDynamicGroupMaxItems(undefined);
    setDynamicGroupSensitivity('internal');
  };

  const closeEdit = () => {
    setShowNameError(false);
    setIsNewTemplate(false);
    setEditingTemplate(null);
    setEditName('');
    setEditCategory('identity');
    setEditIconId('document');
    setEditContractTypeId('');
    setEditProperties([]);
    setDynamicGroupEnabled(false);
    setDynamicGroupAllowedTypes(undefined);
    setDynamicGroupMaxItems(undefined);
    setDynamicGroupSensitivity('internal');
  };

  const saveEdit = async () => {
    const name = editName.trim();
    if (!name) {
      showToast({ type: 'warning', message: t('common:name_required', { defaultValue: '请输入模板名称' }) });
      setShowNameError(true);
      return;
    }
    setShowNameError(false);
    // 保存前：对 contractField: true 但尚无 contractBindings 的字段，自动推导并持久化
    const finalProperties = editProperties.map((p) => {
      if (
        p.contractField &&
        (!p.contractBindings || p.contractBindings.length === 0) &&
        editContractTypeId
      ) {
        const derived = deriveContractBindings(editContractTypeId, p.id, installedPlugins);
        if (derived.length > 0) {
          return { ...p, contractBindings: derived };
        }
      }
      return p;
    });
    try {
      if (isNewTemplate) {
        await createTemplate(
          name,
          editIconId,
          editCategory,
          finalProperties,
          editContractTypeId || undefined,
        );
        await loadTemplates();
        closeEdit();
      } else if (editingTemplate) {
        await updateTemplate(editingTemplate.id, {
          name: name || editingTemplate.name,
          iconId: editIconId,
          category: editCategory,
          properties: finalProperties,
          contractTypeId: editContractTypeId || undefined,
        });
        closeEdit();
      }
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:save_failed')}: ${e}` });
    }
  };

  const updatePropertyName = (index: number, newName: string) => {
    setEditProperties((prev) => prev.map((p, i) => (i === index ? { ...p, name: newName } : p)));
  };

  const updatePropertyType = (index: number, newType: PropertyType) => {
    setEditProperties((prev) =>
      prev.map((p, i) => {
        if (i !== index) return p;
        const next: TemplateProperty = { ...p, type: newType };
        // 切换为/退出 dynamic_group 时清理或初始化相关配置
        delete (next as Partial<TemplateProperty>).allowedTypes;
        delete (next as Partial<TemplateProperty>).maxItems;
        return next;
      }),
    );
  };

  // 动态字段组回调：同步 editProperties 中的 dynamic_group 字段
  const handleDynamicGroupEnabledChange = (enabled: boolean) => {
    setDynamicGroupEnabled(enabled);
    if (enabled) {
      // 追加一个 dynamic_group 字段
      const newDg: TemplateProperty = {
        id: crypto.randomUUID(),
        name: '__dynamic_group__',
        type: 'dynamic_group',
        sensitivityLevel: dynamicGroupSensitivity,
        allowedTypes: dynamicGroupAllowedTypes,
        maxItems: dynamicGroupMaxItems,
      };
      setEditProperties((prev) => [...prev, newDg]);
    } else {
      // 移除 dynamic_group 字段
      setEditProperties((prev) => prev.filter((p) => p.type !== 'dynamic_group'));
    }
  };

  const handleDynamicGroupAllowedTypesChange = (types: PropertyType[]) => {
    setDynamicGroupAllowedTypes(types);
    setEditProperties((prev) =>
      prev.map((p) =>
        p.type === 'dynamic_group'
          ? { ...p, allowedTypes: types.length > 0 ? types : undefined }
          : p,
      ),
    );
  };

  const handleDynamicGroupMaxItemsChange = (maxItems: number | undefined) => {
    setDynamicGroupMaxItems(maxItems);
    setEditProperties((prev) =>
      prev.map((p) => (p.type === 'dynamic_group' ? { ...p, maxItems } : p)),
    );
  };

  const handleDynamicGroupSensitivityChange = (level: SensitivityLevel) => {
    setDynamicGroupSensitivity(level);
    setEditProperties((prev) =>
      prev.map((p) => (p.type === 'dynamic_group' ? { ...p, sensitivityLevel: level } : p)),
    );
  };

  const updatePropertySensitivity = (
    index: number,
    level: 'public' | 'internal' | 'sensitive' | 'critical',
  ) => {
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

  const updatePropertyContractBindings = (index: number, bindings: ContractRoleBinding[]) => {
    setEditProperties((prev) =>
      prev.map((p, i) =>
        i === index ? { ...p, contractBindings: bindings.length > 0 ? bindings : undefined } : p,
      ),
    );
  };

  const removeProperty = async (index: number) => {
    const prop = editProperties[index];
    if (!prop || !editingTemplate) return;
    if (isNewTemplate) {
      setEditProperties((prev) => prev.filter((_, i) => i !== index));
      return;
    }
    try {
      const usage = await checkFieldUsage(editingTemplate.id, prop.id);
      if (usage.active > 0 || usage.softDeleted > 0) {
        requestConfirm(
          t('settings:confirm_deprecate_title'),
          t('settings:confirm_deprecate_body', {
            activeCount: usage.active,
            softDeletedCount: usage.softDeleted,
          }),
          () => {
            setEditProperties((prev) =>
              prev.map((p, i) =>
                i === index ? { ...p, deprecatedAt: new Date().toISOString() } : p,
              ),
            );
            setFieldUsageMap((prev) => ({ ...prev, [prop.id]: usage }));
          },
          { confirmLabel: t('common:confirm'), cancelLabel: t('common:cancel') },
        );
      } else {
        setEditProperties((prev) => prev.filter((_, i) => i !== index));
      }
    } catch {
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
      name: t('settings:new_field_name', { defaultValue: '新字段' }),
      type: newFieldType,
      sensitivityLevel: 'internal',
      allowedTypes: undefined,
      maxItems: undefined,
    };
    setEditProperties((prev) => [...prev, newProp]);
  };

  return {
    editingTemplate,
    isNewTemplate,
    editName,
    editCategory,
    editIconId,
    editContractTypeId,
    editProperties,
    newFieldType,
    dynamicGroupEnabled,
    dynamicGroupAllowedTypes,
    dynamicGroupMaxItems,
    dynamicGroupSensitivity,
    showDeprecated,
    fieldUsageMap,
    showNameError,
    confirmDialog,
    // 编辑器基础输入 setter（TemplateEditorModal 透传用）
    setEditName,
    setEditCategory,
    setEditIconId,
    setEditContractTypeId,
    setNewFieldType,
    openEdit,
    openCreate,
    closeEdit,
    saveEdit,
    updatePropertyName,
    updatePropertyType,
    updatePropertySensitivity,
    updatePropertyOptions,
    updatePropertyContractBindings,
    removeProperty,
    restoreProperty,
    permanentlyRemoveProperty,
    addProperty,
    handleDynamicGroupEnabledChange,
    handleDynamicGroupAllowedTypesChange,
    handleDynamicGroupMaxItemsChange,
    handleDynamicGroupSensitivityChange,
    toggleShowDeprecated: () => setShowDeprecated((v) => !v),
  };
}
