import { useTranslation } from 'react-i18next';
import { Dialog } from '@/components/ui/Dialog';
import { TemplateEditor } from '@/components/template/TemplateEditor';
import type { useTemplateEditor } from '@/hooks/useTemplateEditor';

interface TemplateEditorModalProps {
  /** useTemplateEditor 收敛的全部编辑器状态与回调（P224-③ 拆分，数据经编排层透传） */
  editor: ReturnType<typeof useTemplateEditor>;
}

/**
 * 模板编辑/新建对话框（纯展示，P224-③ 拆分）：
 * Dialog 外壳 + TemplateEditor 全量 props 透传，标题随 isNewTemplate 切换。
 */
export function TemplateEditorModal({ editor }: TemplateEditorModalProps) {
  const { t } = useTranslation(['settings']);
  const {
    editingTemplate,
    isNewTemplate,
    editName,
    editCategory,
    editIconId,
    editContractTypeId,
    editProperties,
    newFieldType,
    showDeprecated,
    fieldUsageMap,
    showNameError,
    dynamicGroupEnabled,
    dynamicGroupAllowedTypes,
    dynamicGroupMaxItems,
    dynamicGroupSensitivity,
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
    toggleShowDeprecated,
  } = editor;

  return (
    <Dialog
      isOpen={!!editingTemplate}
      onClose={closeEdit}
      title={
        isNewTemplate
          ? t('settings:new_template', { defaultValue: '新建模板' })
          : t('settings:edit_template', { defaultValue: '编辑模板' })
      }
    >
      <TemplateEditor
        editingTemplate={editingTemplate}
        editName={editName}
        editCategory={editCategory}
        editIconId={editIconId}
        editContractTypeId={editContractTypeId}
        editProperties={editProperties}
        newFieldType={newFieldType}
        showDeprecated={showDeprecated}
        fieldUsageMap={fieldUsageMap}
        onEditNameChange={editor.setEditName}
        onEditCategoryChange={editor.setEditCategory}
        onEditIconIdChange={editor.setEditIconId}
        onContractTypeIdChange={editor.setEditContractTypeId}
        onNewFieldTypeChange={editor.setNewFieldType}
        onAddProperty={addProperty}
        onUpdatePropertyName={updatePropertyName}
        onUpdatePropertyType={updatePropertyType}
        onUpdatePropertySensitivity={updatePropertySensitivity}
        onUpdatePropertyOptions={updatePropertyOptions}
        onUpdatePropertyContractBindings={updatePropertyContractBindings}
        onRemoveProperty={removeProperty}
        onRestoreProperty={restoreProperty}
        onPermanentlyRemoveProperty={permanentlyRemoveProperty}
        onToggleShowDeprecated={toggleShowDeprecated}
        onSave={saveEdit}
        onClose={closeEdit}
        nameError={showNameError}
        dynamicGroupEnabled={dynamicGroupEnabled}
        dynamicGroupAllowedTypes={dynamicGroupAllowedTypes}
        dynamicGroupMaxItems={dynamicGroupMaxItems}
        dynamicGroupSensitivity={dynamicGroupSensitivity}
        onDynamicGroupEnabledChange={handleDynamicGroupEnabledChange}
        onDynamicGroupAllowedTypesChange={handleDynamicGroupAllowedTypesChange}
        onDynamicGroupMaxItemsChange={handleDynamicGroupMaxItemsChange}
        onDynamicGroupSensitivityChange={handleDynamicGroupSensitivityChange}
      />
    </Dialog>
  );
}
