import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { ObjectEditLockBadge } from '@/components/object/ObjectEditLockBadge';
import { Lock, LockOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ObjectTemplateSelector } from '@/components/editor/ObjectTemplateSelector';
import { ObjectFieldList } from '@/components/editor/ObjectFieldList';
import { AndroidObjectDestination } from '@/components/android/AndroidObjectDestination';
import { isAndroidSync } from '@/lib/platform';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { useAuthStore } from '@/stores/authStore';
import { ContentLoadingSkeleton } from '@/components/ui/ContentLoadingSkeleton';
import { useObjectEditorPage } from './useObjectEditorPage';
import styles from './ObjectEditorPage.module.css';

export function ObjectEditorPage() {
  const { objectId } = useParams();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { t } = useTranslation('common');
  const section = searchParams.get('section') || '';
  const parentId = searchParams.get('parentId') || '';
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  if (isAndroidSync() && !objectId && !section && !parentId) {
    return (
      <PageShell title={t('new_object')} onBack={() => navigate(-1)}>
        <AndroidObjectDestination />
      </PageShell>
    );
  }
  // 同账户编辑对象时保留页面框架；归属或账户变化仍隔离整个表单。
  return (
    <ObjectEditorForm key={JSON.stringify([accountId, Boolean(objectId), section, parentId])} />
  );
}

function ObjectEditorForm() {
  const navigate = useNavigate();
  const useTitlebarActions = useNativeWindowStore((s) => s.isWindows || s.isMacOS);
  const { t } = useTranslation(['common', 'editor', 'navigation']);
  const {
    objectId,
    isNew,
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
    historyRevision,
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
    isEditLocked,
    handleToggleEditLock,
  } = useObjectEditorPage();

  const [loadingNotice, setLoadingNotice] = useState(false);
  useEffect(() => {
    if (!isLoadingObject) {
      setLoadingNotice(false);
      return;
    }
    const timer = setTimeout(() => setLoadingNotice(true), 250);
    return () => clearTimeout(timer);
  }, [isLoadingObject]);
  const showLoadingNotice = isLoadingObject && loadingNotice;

  const showForm = isNew ? Boolean(selectedType) : Boolean(currentObject);
  const showActions = !isNew || showForm;
  const objectInspector = useMemo(
    () =>
      objectId
        ? {
            objectId,
            ready: isObjectReady,
            failed: objectLoadFailed,
            objectName: isObjectReady ? (currentObject?.name ?? '') : '',
            typeId: isObjectReady ? currentObject?.typeId : undefined,
            historyRevision,
          }
        : undefined,
    [
      objectId,
      isObjectReady,
      objectLoadFailed,
      currentObject?.name,
      currentObject?.typeId,
      historyRevision,
    ],
  );
  const formActions = (
    <>
      {!isEditLocked && (
        <>
          <Button
            variant="secondary"
            size={useTitlebarActions ? 'sm' : 'md'}
            onClick={() => navigate(-1)}
            disabled={isSaving}
          >
            {t('common:cancel')}
          </Button>
          <Button
            variant="primary"
            size={useTitlebarActions ? 'sm' : 'md'}
            onClick={handleSave}
            loading={isSaving}
            disabled={!isObjectReady || isSaving}
            className={styles.saveButton}
            data-object-loading={isLoadingObject && !showLoadingNotice ? 'brief' : undefined}
          >
            {t('common:save')}
          </Button>
        </>
      )}
      {!isNew && (
        <Button
          variant="secondary"
          size={useTitlebarActions ? 'sm' : 'md'}
          onClick={handleToggleEditLock}
          loading={isSaving}
          disabled={!isObjectReady || isSaving}
        >
          {isEditLocked ? (
            <LockOpen size={16} aria-hidden="true" />
          ) : (
            <Lock size={16} aria-hidden="true" />
          )}
          {t(isEditLocked ? 'common:object_unlock' : 'common:object_lock')}
        </Button>
      )}
    </>
  );

  return (
    <PageShell
      title={isNew ? t('common:new_object') : t('common:edit_object')}
      onBack={handleBack}
      continuityScope={isNew ? undefined : '/editor/:objectId'}
      primaryActions={useTitlebarActions && showActions ? formActions : undefined}
      windowsContentToolbar
      objectInspector={objectInspector}
    >
      <PageContainer variant="xs" gap="default" className={styles.editor}>
        {showLoadingNotice && (
          <span className={styles.loadingStatus} role="status" data-editor-loading-notice>
            {t('common:loading')}
          </span>
        )}
        {objectLoadFailed && (
          <div role="alert" className={styles.loadError}>
            {t('common:object_load_failed')}
            <Button size="sm" variant="secondary" onClick={retryObjectLoad}>
              {t('common:retry')}
            </Button>
          </div>
        )}
        <div
          className={styles.formBody}
          data-object-editor-form
          data-edit-locked={isEditLocked || undefined}
          aria-busy={isLoadingObject}
          inert={!isObjectReady}
        >
          <div className={styles.formBody} key={currentObject?.id ?? 'new'}>
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

            {showForm && (
              <>
                <Card>
                  <Input
                    label={t('common:object_name')}
                    aria-label={t('common:object_name')}
                    value={name}
                    readOnly={isEditLocked || isSaving}
                    onChange={(e) => setName(e.target.value)}
                    placeholder={t('common:object_name_placeholder')}
                  />
                  {!isNew && objectId && (
                    <div className={styles.objectStatus}>
                      <ObjectEditLockBadge
                        objectId={objectId}
                        properties={currentObject?.properties}
                      />
                    </div>
                  )}
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
                  showSuggestions={!isEditLocked}
                  readOnly={isEditLocked || isSaving}
                />
              </>
            )}
            {!isNew && !currentObject && <ContentLoadingSkeleton />}
          </div>
        </div>
        {!useTitlebarActions && showActions && (
          <div className={styles.formActions}>{formActions}</div>
        )}
      </PageContainer>
    </PageShell>
  );
}
