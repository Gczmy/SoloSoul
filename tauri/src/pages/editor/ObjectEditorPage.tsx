import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useTranslation } from 'react-i18next';
import { ObjectTemplateSelector } from '@/components/editor/ObjectTemplateSelector';
import { ObjectFieldList } from '@/components/editor/ObjectFieldList';
import { AndroidObjectDestination } from '@/components/android/AndroidObjectDestination';
import { isAndroidSync } from '@/lib/platform';
import { useObjectEditorPage } from './useObjectEditorPage';
import styles from './ObjectEditorPage.module.css';

export function ObjectEditorPage() {
  const { objectId } = useParams();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { t } = useTranslation('common');
  const section = searchParams.get('section') || '';
  const parentId = searchParams.get('parentId') || '';
  if (isAndroidSync() && !objectId && !section && !parentId) {
    return (
      <PageShell title={t('new_object')} onBack={() => navigate(-1)}>
        <AndroidObjectDestination />
      </PageShell>
    );
  }
  // 页面归属变化时重新挂载表单，避免上一页面的模板、字段和校验状态串入。
  return <ObjectEditorForm key={JSON.stringify([objectId, section, parentId])} />;
}

function ObjectEditorForm() {
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
    name,
    setName,
    fields,
    displayFields,
    values,
    handleFieldChange,
    validationErrors,
    handleClearError,
    getSensitivity,
    fieldSuggestions,
    handleSave,
    handleBack,
    isSaving,
  } = useObjectEditorPage();

  return (
    <PageShell title={isNew ? t('common:new_object') : t('common:edit_object')} onBack={handleBack}>
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
        />

        {!isNew && loadedFor !== objectId
          ? null
          : (selectedType || !isNew) && (
              <>
                <Card>
                  <Input
                    label={t('common:object_name')}
                    aria-label={t('common:object_name')}
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
                  suggestions={fieldSuggestions}
                />
                <div className={styles.formActions}>
                  <Button variant="secondary" onClick={() => navigate(-1)}>
                    {t('common:cancel')}
                  </Button>
                  <Button variant="primary" onClick={handleSave} loading={isSaving}>
                    {t('common:save')}
                  </Button>
                </div>
              </>
            )}
      </PageContainer>
    </PageShell>
  );
}
