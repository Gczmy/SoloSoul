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
import { useObjectStore, ObjectData } from '@/stores/objectStore';
import { useSensitivityStore, SensitivityLevel } from '@/stores/sensitivityStore';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { useToastError } from '@/hooks/useToastError';

// Each template belongs to a workspace section.
// collectionType is the section (for filtering), not the template name.
type TemplateCategory = 'identity' | 'travel' | 'financial' | 'professional';

const templateMeta: Record<string, { category: TemplateCategory; label: string }> = {
  identity:    { category: 'identity',     label: 'Identity' },
  passport:    { category: 'travel',       label: 'Passport' },
  visa:        { category: 'travel',       label: 'Visa' },
  bank:        { category: 'financial',    label: 'Bank Account' },
  card:        { category: 'financial',    label: 'Card' },
  education:   { category: 'professional', label: 'Education' },
  employment:  { category: 'professional', label: 'Employment' },
};

const objectTemplates: Record<string, { key: string; label: string; type: string; sensitive?: boolean }[]> = {
  identity: [
    { key: 'fullName', label: 'Full Name', type: 'text' },
    { key: 'dateOfBirth', label: 'Date of Birth', type: 'date' },
    { key: 'nationality', label: 'Nationality', type: 'text' },
    { key: 'idNumber', label: 'ID Number', type: 'text', sensitive: true },
    { key: 'email', label: 'Email', type: 'email' },
    { key: 'phone', label: 'Phone', type: 'tel' },
  ],
  passport: [
    { key: 'fullName', label: 'Full Name', type: 'text' },
    { key: 'passportNumber', label: 'Passport Number', type: 'text', sensitive: true },
    { key: 'nationality', label: 'Nationality', type: 'text' },
    { key: 'dateOfBirth', label: 'Date of Birth', type: 'date' },
    { key: 'issueDate', label: 'Issue Date', type: 'date' },
    { key: 'expiryDate', label: 'Expiry Date', type: 'date' },
  ],
  visa: [
    { key: 'country', label: 'Country', type: 'text' },
    { key: 'visaType', label: 'Visa Type', type: 'text' },
    { key: 'number', label: 'Visa Number', type: 'text', sensitive: true },
    { key: 'issueDate', label: 'Issue Date', type: 'date' },
    { key: 'expiryDate', label: 'Expiry Date', type: 'date' },
  ],
  bank: [
    { key: 'bankName', label: 'Bank Name', type: 'text' },
    { key: 'accountNumber', label: 'Account Number', type: 'text', sensitive: true },
    { key: 'accountType', label: 'Account Type', type: 'text' },
    { key: 'currency', label: 'Currency', type: 'text' },
  ],
  card: [
    { key: 'cardNumber', label: 'Card Number', type: 'text', sensitive: true },
    { key: 'cardType', label: 'Card Type', type: 'text' },
    { key: 'holderName', label: 'Holder Name', type: 'text' },
    { key: 'expiryDate', label: 'Expiry Date', type: 'date' },
  ],
  education: [
    { key: 'institution', label: 'Institution', type: 'text' },
    { key: 'degree', label: 'Degree', type: 'text' },
    { key: 'field', label: 'Field', type: 'text' },
    { key: 'startDate', label: 'Start Date', type: 'date' },
    { key: 'endDate', label: 'End Date', type: 'date' },
  ],
  employment: [
    { key: 'company', label: 'Company', type: 'text' },
    { key: 'position', label: 'Position', type: 'text' },
    { key: 'startDate', label: 'Start Date', type: 'date' },
    { key: 'endDate', label: 'End Date', type: 'date' },
  ],
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
  const { getObject, createObject, updateObject, currentObject } = useObjectStore();
  const { map: sensitivityMap, loadMap } = useSensitivityStore();
  const { onError, onSuccess } = useToastError();

  // Load sensitivity map for field-level indicators
  useEffect(() => { loadMap(); }, []);

  /** Resolve sensitivity level for a property field. */
  const getSensitivity = (fieldKey: string): SensitivityLevel => {
    // Build field ID like "travel.passport_number" or "identity.full_name"
    const fieldId = `${collectionType || sectionParam || 'collection'}.${fieldKey}`;
    if (sensitivityMap?.entries?.[fieldId]) {
      return sensitivityMap.entries[fieldId];
    }
    // Fallback: check if the field key alone matches
    for (const [id, level] of Object.entries(sensitivityMap?.entries || {})) {
      if (id.endsWith(`.${fieldKey}`)) return level;
    }
    return 'internal'; // default level
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
  const [values, setValues] = useState<Record<string, string>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [dataLoaded, setDataLoaded] = useState(false);

  const fields = objectTemplates[selectedType] || [];

  // Determine collectionType
  const collectionType = isNew
    ? sectionParam || (selectedType ? (templateMeta[selectedType]?.category || selectedType) : '')
    : currentObject?.collectionType || '';

  // Load existing object and populate form
  useEffect(() => {
    if (!isNew && objectId && accountId) {
      getObject(accountId, objectId).catch((e) => onError(e, t('common:object_load_failed')));
    }
  }, [objectId, accountId]);

  // When currentObject loads (for editing), populate the form
  useEffect(() => {
    if (isNew || !currentObject || dataLoaded) return;
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
    // Try to detect template from property keys
    const propKeys = Object.keys(vals);
    let bestMatch = '';
    let bestScore = 0;
    for (const [tplName, tplFields] of Object.entries(objectTemplates)) {
      const tplKeys = tplFields.map((f) => f.key);
      const matchCount = tplKeys.filter((k) => propKeys.includes(k)).length;
      if (matchCount > bestScore && matchCount >= tplKeys.length * 0.5) {
        bestScore = matchCount;
        bestMatch = tplName;
      }
    }
    if (bestMatch) {
      setSelectedType(bestMatch);
    }
    setDataLoaded(true);
  }, [currentObject, isNew, dataLoaded]);

  const handleSave = async () => {
    if (!accountId) return;
    setIsSaving(true);
    try {
      if (isNew) {
        await createObject({
          accountId,
          name: name || templateMeta[selectedType]?.label || 'Untitled',
          collectionType,
          properties: values as unknown as Record<string, unknown>,
          parentId,
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
              {visibleTemplates.map((type) => (
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
                  {t(`editor:templates.${type}`, type)}
                </button>
              ))}
            </div>
          </Card>
        )}

        {!isNew && collectionType && (
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{t('common:object_type')}:</span>
              <span style={{
                fontSize: 12, fontWeight: 500, padding: '2px 8px', borderRadius: 4,
                background: 'var(--bg-toolbar)', color: 'var(--text-primary)',
              }}>
                {collectionType}
              </span>
              {selectedType && (
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                  · {t('editor:templates.' + selectedType, t('editor:templates.' + selectedType.toLowerCase(), selectedType))}
                </span>
              )}
            </div>
          </Card>
        )}

        {(selectedType || !isNew) && (
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
                {fields.map((field) => {
                    const sensitivity = getSensitivity(field.key);
                    const fieldLabel = t(`editor:fields.${field.key}`, field.label);
                    return (
                  <div key={field.key}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                      <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{fieldLabel}</label>
                      <SensitivityBadge level={sensitivity} />
                    </div>
                    <Input
                      type={field.type}
                      value={values[field.key] || ''}
                      onChange={(e) => setValues((v) => ({ ...v, [field.key]: e.target.value }))}
                      placeholder={fieldLabel}
                    />
                  </div>
                    );
                  })}
              </div>
            </Card>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              {!isNew && objectId && (
                <>
                <Button variant="secondary" onClick={() => navigate(`/history?objectId=${objectId}`)}>
                  History
                </Button>
                <Button variant="secondary" onClick={async () => {
                  const name = prompt('Template name:', currentObject?.name || '');
                  if (name && objectId) {
                    try {
                      await invoke('template_save_from_object', { objectId, templateName: name });
                      alert('Template saved');
                    } catch (e) { alert('Failed: ' + e); }
                  }
                }}>
                  Save as Template
                </Button>
                <Button variant="secondary" onClick={async () => {
                  const path = await open({ multiple: false, title: 'Select file to attach' });
                  if (path && typeof path === 'string' && objectId) {
                    try {
                      await invoke('attachment_save', { objectId, meta: {
                        id: crypto.randomUUID(), objectId,
                        fileName: path.split('/').pop() || 'file',
                        mimeType: 'application/octet-stream', sizeBytes: 0, createdAt: new Date().toISOString(),
                      }});
                      alert('Attachment added');
                    } catch (e) { alert('Failed: ' + e); }
                  }
                }}>
                  Add Attachment
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
