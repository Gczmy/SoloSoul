import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useToastError } from '@/hooks/useToastError';
import { getBiometricErrorMessage } from '@/lib/biometricError';
import { invoke } from '@tauri-apps/api/core';
import { Fingerprint, ShieldCheck } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

interface BiometricSectionProps {
  accountId: string;
}

export function BiometricSection({ accountId }: BiometricSectionProps) {
  const { t } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();

  const biometricEnabled = useSettingsStore((s) => s.settings.biometricEnabled);
  const updateSetting = useSettingsStore((s) => s.updateSetting);

  const [bioAvailable, setBioAvailable] = useState<{
    available: boolean;
    biometryType?: string;
  } | null>(null);
  const [bioLoading, setBioLoading] = useState(false);
  const [showBioPwDialog, setShowBioPwDialog] = useState(false);
  const [bioPw, setBioPw] = useState('');
  const [bioAction, setBioAction] = useState<'enable' | 'disable' | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Password hint for the biometric verification dialog
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const passwordHint = currentAccount?.passwordHint || null;

  useEffect(() => {
    invoke<{ available: boolean; biometryType?: string }>('biometric_check_availability', {
      accountId,
    })
      .then(setBioAvailable)
      .catch(() => setBioAvailable({ available: false }));
  }, [accountId]);

  const biometryType =
    bioAvailable?.biometryType === 'touchId'
      ? 'Touch ID'
      : bioAvailable?.biometryType === 'faceId'
        ? 'Face ID'
        : bioAvailable?.biometryType === 'windowsHello'
          ? 'Windows Hello'
          : 'Touch ID';

  const handleBioToggle = () => {
    setBioAction(biometricEnabled ? 'disable' : 'enable');
    setBioPw('');
    setShowBioPwDialog(true);
  };

  const handleBioConfirm = async () => {
    if (!bioPw) return;
    setBioLoading(true);
    setError(null);
    try {
      const rawType = bioAvailable?.biometryType || 'unknown';
      if (bioAction === 'enable') {
        await invoke('biometric_save_credential', {
          accountId,
          password: bioPw,
          location: 'settings_page',
          action: 'enable',
          biometryType: rawType,
        });
        await updateSetting(accountId, 'biometricEnabled', true);
        onSuccess(t('settings:biometric_enabled_toast', { type: biometryType }));
      } else {
        await invoke('biometric_delete_credential', {
          accountId,
          password: bioPw,
          location: 'settings_page',
          action: 'disable',
          biometryType: rawType,
        });
        await updateSetting(accountId, 'biometricEnabled', false);
        onSuccess(t('settings:biometric_disabled_toast', { type: biometryType }));
      }
      setShowBioPwDialog(false);
    } catch (e) {
      const msg = String(e);
      if (
        msg.toLowerCase().includes('invalid password') ||
        msg.toLowerCase().includes('incorrect')
      ) {
        setError(t('settings:current_password_incorrect'));
      } else {
        setError(getBiometricErrorMessage(e, t));
      }
    } finally {
      setBioLoading(false);
    }
  };

  const handleBioTest = async () => {
    try {
      await invoke('biometric_test', { accountId });
      onSuccess(t('settings:biometric_test_success', { type: biometryType }));
    } catch (e) {
      onError(
        getBiometricErrorMessage(e, t),
        t('settings:biometric_test_failed', { type: biometryType }),
      );
    }
  };

  if (bioAvailable === null) return null;

  return (
    <>
      <Card>
        <h3
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 600,
            marginBottom: 4,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <Fingerprint size={ICON_SIZE.lg} />
          {t('settings:biometric_title')}
        </h3>
        {!bioAvailable.available ? (
          <div
            style={{
              padding: '12px 14px',
              borderRadius: 8,
              background: 'color-mix(in srgb, var(--accent-primary) 6%, transparent)',
              border: '1px solid color-mix(in srgb, var(--accent-primary) 20%, transparent)',
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              lineHeight: 1.5,
            }}
          >
            {t('settings:biometric_unavailable_desc') ??
              '当前设备未设置或不支持生物识别（Touch ID / Face ID）。请先在系统设置中添加指纹或面容，然后重新打开此页面。'}
          </div>
        ) : (
          <>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                marginBottom: 12,
              }}
            >
              {t('settings:biometric_desc', { type: biometryType })}
            </p>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 'var(--text-body)' }}>
                {t('settings:biometric_toggle_label', { type: biometryType })}
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                {biometricEnabled && (
                  <Button variant="secondary" size="sm" onClick={handleBioTest}>
                    <ShieldCheck size={ICON_SIZE.sm} style={{ marginRight: 4 }} />
                    {t('settings:biometric_test_button', { type: biometryType })}
                  </Button>
                )}
                <label
                  style={{
                    position: 'relative',
                    display: 'inline-block',
                    width: 44,
                    height: 24,
                    cursor: 'pointer',
                  }}
                >
                  <input
                    type="checkbox"
                    checked={biometricEnabled}
                    onChange={handleBioToggle}
                    style={{ opacity: 0, width: 0, height: 0 }}
                  />
                  <span
                    style={{
                      position: 'absolute',
                      inset: 0,
                      background: biometricEnabled
                        ? 'var(--accent-primary)'
                        : 'var(--border-subtle)',
                      borderRadius: 12,
                      transition: '0.2s',
                    }}
                  />
                  <span
                    style={{
                      position: 'absolute',
                      top: 2,
                      left: biometricEnabled ? 22 : 2,
                      width: 20,
                      height: 20,
                      borderRadius: '50%',
                      background: 'white',
                      transition: '0.2s',
                      boxShadow: '0 1px 3px rgba(0,0,0,0.2)',
                    }}
                  />
                </label>
              </div>
            </div>
          </>
        )}
      </Card>

      {/* Biometric password verification dialog */}
      {showBioPwDialog && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 'var(--z-modal-important)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.45)',
            backdropFilter: 'blur(6px)',
          }}
        >
          <div
            style={{
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontFamily: 'inherit',
              borderRadius: 16,
              padding: '28px 32px',
              maxWidth: 380,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            <h3
              style={{
                fontSize: 'var(--text-md)',
                fontWeight: 600,
                marginBottom: 12,
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              <Fingerprint size={ICON_SIZE.xl} />
              {bioAction === 'enable'
                ? t('settings:biometric_enable_prompt', { type: biometryType })
                : t('settings:biometric_disable_prompt', { type: biometryType })}
            </h3>
            <SecurePasswordInput
              label={t('common:current_password')}
              value={bioPw}
              onChange={(v) => {
                setBioPw(v);
                setError(null);
              }}
              placeholder={t('common:password_placeholder')}
              autoComplete="current-password"
              showHintButton={true}
              hint={passwordHint}
              onEnter={handleBioConfirm}
            />
            {error && (
              <div
                style={{
                  color: '#dc2626',
                  fontSize: 'var(--text-body-sm)',
                  padding: '4px 0',
                  marginTop: 8,
                }}
              >
                {error}
              </div>
            )}
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 16 }}>
              <Button
                variant="secondary"
                onClick={() => {
                  setShowBioPwDialog(false);
                  setError(null);
                }}
              >
                {t('common:cancel')}
              </Button>
              <Button onClick={handleBioConfirm} loading={bioLoading} disabled={!bioPw}>
                {t('common:confirm')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
