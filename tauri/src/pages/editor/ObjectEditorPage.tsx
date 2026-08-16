import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useTranslation } from 'react-i18next';
import { ObjectTemplateSelector } from '@/components/editor/ObjectTemplateSelector';
import { ObjectFieldList } from '@/components/editor/ObjectFieldList';
import { useObjectEditorPage } from './useObjectEditorPage';
import styles from './ObjectEditorPage.module.css';

export function ObjectEditorPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'editor', 'navigation']);
  const {
    isNew,
    objectId,
    loadedFor,
    visibleTemplates,
    selectedType,
    setSelectedType,
    templateMeta,
    userTemplates,
    typeId,
    currentObject,
    contractTypeId,
    customPages,
    sectionParam,
    name,
    setName,
    fields,
    displayFields,
    values,
    handleFieldChange,
    validationErrors,
    handleClearError,
    getSensitivity,
    handleSave,
    handleBack,
    isSaving,
  } = useObjectEditorPage();

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
