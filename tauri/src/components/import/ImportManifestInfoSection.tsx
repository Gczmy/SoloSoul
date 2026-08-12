import type { TFunction } from 'i18next';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { TransferButton } from '@/components/transfer/TransferButton';
import type { DecryptedImportPreview, ImportPreview } from '@/types/exportImport';

/**
 * ImportSection 的「清单信息 + 密码解密」区（P046 拆分：展示子组件）。
 */
export function ImportManifestInfoSection({
  importPreview,
  importPw,
  isDecrypting,
  decryptedPreview,
  onSetImportPw,
  onDecrypt,
  t,
}: {
  importPreview: ImportPreview;
  importPw: string;
  isDecrypting: boolean;
  decryptedPreview: DecryptedImportPreview | null;
  onSetImportPw: (v: string) => void;
  onDecrypt: () => void;
  t: TFunction;
}) {
  return (
    <>
      <div
        style={{
          fontSize: 'var(--text-body-sm)',
          display: 'flex',
          flexDirection: 'column',
          gap: 6,
        }}
      >
        <p>
          {t('settings:version')}: {importPreview.version}
        </p>
        <p>
          {t('settings:export_time')}: {importPreview.exportTime || t('settings:unknown')}
        </p>
        <p>{t('settings:objects_count', { n: importPreview.objectCount })}</p>
        {importPreview.hasAttachments && (
          <p style={{ color: 'var(--accent-primary)' }}>{t('settings:includes_attachments')}</p>
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
            fontSize: 'var(--text-body-sm)',
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
          <TransferButton
            onClick={onDecrypt}
            disabled={!importPw || isDecrypting}
            busy={isDecrypting}
          >
            {isDecrypting
              ? t('common:loading', { defaultValue: '...' })
              : t('settings:decrypt_and_preview')}
          </TransferButton>
        </div>
      )}
    </>
  );
}
