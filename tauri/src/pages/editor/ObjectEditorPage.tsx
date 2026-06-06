import { useState, useEffect, useMemo } from 'react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore, ObjectData } from '@/stores/objectStore';
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
  const { getObject, createObject, updateObject } = useObjectStore();
  const { onError, onSuccess } = useToastError();

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

  const fields = objectTemplates[selectedType] || [];

  // Determine collectionType from section param (not template type)
  const collectionType = sectionParam || (selectedType ? (templateMeta[selectedType]?.category || selectedType) : '');

  // Load existing object for editing
  useEffect(() => {
    if (!isNew && objectId && accountId) {
      getObject(accountId, objectId).catch((e) => onError(e, 'Failed to load object'));
    }
  }, [objectId, accountId]);

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
        onSuccess('Object created');
      } else {
        await updateObject(objectId!, {
          name: name || 'Untitled',
          properties: values as unknown as Record<string, unknown>,
        });
        onSuccess('Object saved');
      }
      navigate(-1);
    } catch (e) {
      onError(e, 'Failed to save');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <AppShell title={isNew ? 'New Object' : 'Edit Object'} onBack={() => navigate(-1)}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {isNew && (
          <Card>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
              Object Type
              {sectionParam && (
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 8, fontWeight: 400 }}>
                  in {sectionParam.charAt(0).toUpperCase() + sectionParam.slice(1)}
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
                  {templateMeta[type]?.label || type.charAt(0).toUpperCase() + type.slice(1)}
                </button>
              ))}
            </div>
          </Card>
        )}

        {(selectedType || !isNew) && (
          <>
            <Card>
              <Input
                label="Object Name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={templateMeta[selectedType]?.label || 'Enter name'}
              />
            </Card>
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
                {templateMeta[selectedType]?.label || 'Properties'}
              </h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {fields.map((field) => (
                  <div key={field.key}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                      <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{field.label}</label>
                      {field.sensitive && (
                        <span style={{
                          fontSize: 10, padding: '1px 6px', borderRadius: 4,
                          background: 'rgba(196,146,92,0.15)', color: 'var(--accent-warm)',
                        }}>SENSITIVE</span>
                      )}
                    </div>
                    <Input
                      type={field.type}
                      value={values[field.key] || ''}
                      onChange={(e) => setValues((v) => ({ ...v, [field.key]: e.target.value }))}
                      placeholder={field.label}
                    />
                  </div>
                ))}
              </div>
            </Card>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <Button variant="secondary" onClick={() => navigate(-1)}>Cancel</Button>
              <Button onClick={handleSave} loading={isSaving}>Save</Button>
            </div>
          </>
        )}
      </div>
    </AppShell>
  );
}
