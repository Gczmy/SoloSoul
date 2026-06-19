import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import type { ImportStrategy, ImportPreview, DecryptedImportPreview } from '@/types/exportImport';

interface ImportSectionProps {
  importPath: string;
  importPreview: ImportPreview | null;
  importPw: string;
  decryptedPreview: DecryptedImportPreview | null;
  isPreviewing: boolean;
  isDecrypting: boolean;
  isImporting: boolean;
  importStrategy: ImportStrategy;
  importSelections: Map<string, boolean>;
  showStrategySelector: boolean;
  onSetImportPath: (v: string) => void;
  onSetImportPreview: (v: ImportPreview | null) => void;
  onSetDecryptedPreview: (v: DecryptedImportPreview | null) => void;
  onSetImportPw: (v: string) => void;
  onSetShowStrategySelector: (v: boolean) => void;
  onPreview: () => void;
  onDecrypt: () => void;
  onImport: () => void;
  onToggleSelection: (id: string) => void;
  onSetStrategy: (s: ImportStrategy) => void;
}

export function ImportSection({
  importPath,
  importPreview,
  importPw,
  decryptedPreview,
  isPreviewing,
  isDecrypting,
  isImporting,
  importStrategy,
  importSelections,
  showStrategySelector,
  onSetImportPath,
  onSetImportPreview,
  onSetDecryptedPreview,
  onSetImportPw,
  onSetShowStrategySelector,
  onPreview,
  onDecrypt,
  onImport,
  onToggleSelection,
  onSetStrategy,
}: ImportSectionProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <>
      <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
        {t('settings:import_desc')}
      </p>

      {/* File selector */}
      <Card>
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
          {t('settings:select_file')}
        </h3>
        <div style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 8 }}>
          {importPath || t('settings:no_file_selected')}
        </div>
        <Button
          size="sm"
          onClick={async () => {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selected = await open({
              filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
              multiple: false,
            });
            if (selected) {
              onSetImportPath(selected as string);
              onSetImportPreview(null);
              onSetDecryptedPreview(null);
              onSetImportPw('');
              onSetShowStrategySelector(false);
            }
          }}
        >
          {t('settings:select_file')}
        </Button>
        {importPath && !importPreview && (
          <div style={{ marginTop: 8 }}>
            <Button
              size="sm"
              onClick={onPreview}
              loading={isPreviewing}
              disabled={isPreviewing}
            >
              {t('settings:preview')}
            </Button>
          </div>
        )}
      </Card>

      {/* Parsed manifest preview */}
      {importPreview && (
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
            {t('settings:import_preview')}
          </h3>
          <div style={{ fontSize: 13, display: 'flex', flexDirection: 'column', gap: 6 }}>
            <p>
              {t('settings:version')}: {importPreview.version}
            </p>
            <p>
              {t('settings:export_time')}: {importPreview.exportTime || t('settings:unknown')}
            </p>
            <p>{t('settings:objects_count', { n: importPreview.objectCount })}</p>
            {importPreview.hasAttachments && (
              <p style={{ color: 'var(--accent-primary)' }}>
                {t('settings:includes_attachments')}
              </p>
            )}
            {importPreview.extraFiles.length > 0 &&
              importPreview.extraFiles.includes('preferences.enc') && (
                <p style={{ color: 'var(--accent-primary)' }}>
                  {t('settings:includes_preferences')}
                </p>
              )}
          </div>

          {importPreview.passwordHint && (
            <div
              style={{
                marginTop: 8,
                padding: '8px 12px',
                background: 'var(--bg-elevated-hover)',
                borderRadius: 6,
                fontSize: 13,
                color: 'var(--text-secondary)',
              }}
            >
              {t('settings:password_hint_label')}: {importPreview.passwordHint}
            </div>
          )}

          <div style={{ marginTop: 12 }}>
            <SecurePasswordInput
              value={importPw}
              onChange={(v) => onSetImportPw(v)}
              placeholder={t('common:password_placeholder')}
              showHintButton={false}
              onEnter={onDecrypt}
            />
          </div>
          {!decryptedPreview && (
            <div style={{ marginTop: 8 }}>
              <Button
                onClick={onDecrypt}
                loading={isDecrypting}
                disabled={!importPw || isDecrypting}
              >
                {t('settings:decrypt_and_preview')}
              </Button>
            </div>
          )}

          {/* Decrypted preview with conflicts */}
          {decryptedPreview && (
            <>
              <div
                style={{
                  marginTop: 12,
                  borderTop: '1px solid var(--border-subtle)',
                  paddingTop: 12,
                }}
              >
                <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 6 }}>
                  {t('settings:objects_in_package')} ({decryptedPreview.objects.length})
                </h4>

                <div style={{ maxHeight: 240, overflowY: 'auto', fontSize: 13 }}>
                  {decryptedPreview.objects.map((obj) => {
                    const isConflict = decryptedPreview.conflicts.some(
                      (c) => c.objectId === obj.id,
                    );
                    const isSelected = importSelections.get(obj.id) ?? true;
                    return (
                      <div
                        key={obj.id}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 6,
                          padding: '3px 0',
                        }}
                      >
                        <input
                          type="checkbox"
                          checked={isSelected}
                          onChange={() => onToggleSelection(obj.id)}
                          style={{ accentColor: 'var(--accent-primary)' }}
                        />
                        <span style={{ flex: 1 }}>{obj.name}</span>
                        <SensitivityBadge level={obj.sensitivityLevel as SensitivityLevel} />
                        {isConflict && (
                          <span
                            style={{
                              fontSize: 11,
                              color: '#e68a00',
                              border: '1px solid #e68a00',
                              borderRadius: 3,
                              padding: '0 4px',
                            }}
                          >
                            {t('settings:conflict')}
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>

                {decryptedPreview.conflicts.length > 0 && (
                  <div
                    style={{
                      marginTop: 8,
                      padding: '8px 12px',
                      background: '#fff3e0',
                      borderRadius: 6,
                      fontSize: 12,
                      color: '#663c00',
                    }}
                  >
                    {t('settings:conflict_warning', {
                      count: decryptedPreview.conflicts.length,
                    })}
                  </div>
                )}
              </div>

              {!showStrategySelector ? (
                <div style={{ marginTop: 8 }}>
                  <Button
                    onClick={() => onSetShowStrategySelector(true)}
                    size="sm"
                    variant="secondary"
                    style={{ marginRight: 8 }}
                  >
                    {t('settings:advanced_import')}
                  </Button>
                  <Button
                    onClick={onImport}
                    loading={isImporting}
                    disabled={!importPw || isImporting}
                  >
                    {t('settings:quick_import')}
                  </Button>
                </div>
              ) : (
                <div
                  style={{
                    marginTop: 12,
                    padding: 12,
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 8,
                  }}
                >
                  <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>
                    {t('settings:import_strategy_title')}
                  </h4>
                  {(['skipExisting', 'overwrite', 'merge'] as ImportStrategy[]).map((s) => (
                    <label
                      key={s}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        padding: '6px 0',
                        cursor: 'pointer',
                        fontSize: 13,
                      }}
                    >
                      <input
                        type="radio"
                        checked={importStrategy === s}
                        onChange={() => onSetStrategy(s)}
                        style={{ accentColor: 'var(--accent-primary)' }}
                      />
                      <div>
                        <strong>{t(`settings:strategy_${s}`)}</strong>
                        <p style={{ fontSize: 11, color: 'var(--text-tertiary)', margin: 1 }}>
                          {t(`settings:strategy_${s}_desc`)}
                        </p>
                      </div>
                    </label>
                  ))}
                  <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => onSetShowStrategySelector(false)}
                    >
                      {t('common:cancel')}
                    </Button>
                    <Button
                      onClick={onImport}
                      loading={isImporting}
                      disabled={!importPw || isImporting}
                    >
                      {t('settings:import_action')} ({importSelections.size})
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </Card>
      )}
      {importPreview && !decryptedPreview && (
        <p style={{ fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center' }}>
          {t('settings:password_required_for_decrypt')}
        </p>
      )}
    </>
  );
}
