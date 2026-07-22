import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useToastError } from '@/hooks/useToastError';
import { getBiometricErrorMessage } from '@/lib/biometricError';
import { invoke } from '@tauri-apps/api/core';
import { Fingerprint, ShieldCheck, ScanFace, AlertTriangle } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';

interface BiometricSectionProps {
  accountId: string;
}

interface BioAvailability {
  available: boolean;
  biometryType?: string;
  /** Class 3（指纹/强人脸）可用 */
  strongAvailable?: boolean;
  /** Class 2（弱人脸）可用 */
  weakAvailable?: boolean;
  /** strong 槽已保存凭证（Touch ID 开关状态） */
  strongConfigured?: boolean;
  /** weak 槽已保存凭证（Face ID Class 2 开关状态） */
  weakConfigured?: boolean;
}

type BioMode = 'strong' | 'weak';

/** 开关滑块（抽取以避免两行重复） */
function ToggleSwitch({
  checked,
  onChange,
  disabled = false,
}: {
  checked: boolean;
  onChange: () => void;
  disabled?: boolean;
}) {
  return (
    <label
      style={{
        position: 'relative',
        display: 'inline-block',
        width: 44,
        height: 24,
        cursor: disabled ? 'not-allowed' : 'pointer',
        flexShrink: 0,
        opacity: 1,
      }}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={disabled ? () => {} : onChange}
        style={{ opacity: 0, width: 0, height: 0 }}
      />
      <span
        style={{
          position: 'absolute',
          inset: 0,
          background: checked ? 'var(--accent-primary)' : 'var(--border-subtle)',
          borderRadius: 12,
          transition: '0.2s',
        }}
      />
      <span
        style={{
          position: 'absolute',
          top: 2,
          left: checked ? 22 : 2,
          width: 20,
          height: 20,
          borderRadius: '50%',
          background: 'white',
          transition: '0.2s',
          boxShadow: '0 1px 3px rgba(0,0,0,0.2)',
        }}
      />
    </label>
  );
}

export function BiometricSection({ accountId }: BiometricSectionProps) {
  const { t } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();

  const updateSetting = useSettingsStore((s) => s.updateSetting);

  const [bioAvailable, setBioAvailable] = useState<BioAvailability | null>(null);
  const [bioLoading, setBioLoading] = useState(false);
  const [showBioPwDialog, setShowBioPwDialog] = useState(false);
  const [bioPw, setBioPw] = useState('');
  const [bioAction, setBioAction] = useState<'enable' | 'disable' | null>(null);
  const [bioMode, setBioMode] = useState<BioMode>('strong');
  const [error, setError] = useState<string | null>(null);

  // Password hint for the biometric verification dialog
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const passwordHint = currentAccount?.passwordHint || null;

  const refreshAvailability = useCallback(async (): Promise<BioAvailability | null> => {
    try {
      const r = await invoke<BioAvailability>('biometric_check_availability', { accountId });
      setBioAvailable(r);
      return r;
    } catch {
      setBioAvailable({ available: false });
      return null;
    }
  }, [accountId]);

  useEffect(() => {
    refreshAvailability();
  }, [refreshAvailability]);

  const strongType =
    bioAvailable?.biometryType === 'faceId'
      ? 'Face ID'
      : bioAvailable?.biometryType === 'windowsHello'
        ? 'Windows Hello'
        : 'Touch ID';
  const weakType = 'Face ID';
  const modeType = bioMode === 'weak' ? weakType : strongType;

  const handleBioToggle = (mode: BioMode) => {
    const configured =
      mode === 'weak' ? bioAvailable?.weakConfigured : bioAvailable?.strongConfigured;
    setBioMode(mode);
    // 弱人脸（Class 2）不再允许开启，仅已配置的历史用户可单向关闭
    if (mode === 'weak') {
      if (!configured) return;
      setBioAction('disable');
    } else {
      setBioAction(configured ? 'disable' : 'enable');
    }
    setBioPw('');
    setError(null);
    setShowBioPwDialog(true);
  };

  const handleBioConfirm = async () => {
    if (!bioPw) return;
    setBioLoading(true);
    setError(null);
    // 启用生物识别时会触发系统原生验证弹窗（Android Keystore 绑定），
    // 应用会切到后台，暂停自动锁定防止 visibilitychange 误锁
    const { pause, resume } = useAutoLockPauseStore.getState();
    pause();
    try {
      const rawType = bioMode === 'weak' ? 'faceId' : bioAvailable?.biometryType || 'touchId';
      if (bioAction === 'enable') {
        await invoke('biometric_save_credential', {
          accountId,
          password: bioPw,
          location: 'settings_page',
          action: 'enable',
          biometryType: rawType,
          authenticator: bioMode,
        });
        const r = await refreshAvailability();
        // 遗留标志保持同步：任一槽有凭证即为 true
        await updateSetting(accountId, 'biometricEnabled', !!(r?.strongConfigured || r?.weakConfigured));
        onSuccess(t('settings:biometric_enabled_toast', { type: modeType }));
      } else {
        await invoke('biometric_delete_credential', {
          accountId,
          password: bioPw,
          location: 'settings_page',
          action: 'disable',
          biometryType: rawType,
          authenticator: bioMode,
        });
        const r = await refreshAvailability();
        await updateSetting(accountId, 'biometricEnabled', !!(r?.strongConfigured || r?.weakConfigured));
        onSuccess(t('settings:biometric_disabled_toast', { type: modeType }));
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
      resume();
      setBioLoading(false);
    }
  };

  const handleBioTest = async () => {
    // 暂停自动锁定：触发生物识别会将应用切到后台，
    // 若不暂停 visibilitychange handler 会在测试通过后立即锁定 Vault
    const { pause, resume } = useAutoLockPauseStore.getState();
    pause();
    try {
      await invoke('biometric_test', { accountId });
      onSuccess(t('settings:biometric_test_success', { type: strongType }));
    } catch (e) {
      onError(
        getBiometricErrorMessage(e, t),
        t('settings:biometric_test_failed', { type: strongType }),
      );
    } finally {
      resume();
    }
  };

  if (bioAvailable === null) return null;

  const showStrong = !!bioAvailable.strongAvailable;
  const showWeak = !!bioAvailable.weakAvailable;

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
              {t('settings:biometric_desc', { type: strongType })}
            </p>

            {/* Touch ID / 强生物识别 */}
            {showStrong && (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  marginBottom: showWeak ? 12 : 0,
                }}
              >
                <span style={{ fontSize: 'var(--text-body)' }}>
                  {t('settings:biometric_toggle_label', { type: strongType })}
                </span>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  {bioAvailable.strongConfigured && (
                    <Button variant="secondary" size="sm" onClick={handleBioTest}>
                      <ShieldCheck size={ICON_SIZE.sm} style={{ marginRight: 4 }} />
                      {t('settings:biometric_test_button', { type: strongType })}
                    </Button>
                  )}
                  <ToggleSwitch
                    checked={!!bioAvailable.strongConfigured}
                    onChange={() => handleBioToggle('strong')}
                  />
                </div>
              </div>
            )}

            {/* Face ID（Class 2 弱生物识别）—— 置灰不可开启 */}
            {showWeak && (
              <>
                <div
                  style={{
                    padding: '8px 12px',
                    borderRadius: 8,
                    marginBottom: 12,
                    background: 'rgba(212, 133, 10, 0.10)',
                    border: '1px solid rgba(212, 133, 10, 0.25)',
                    fontSize: 'var(--text-caption)',
                    lineHeight: 1.5,
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: 8,
                  }}
                >
                  <AlertTriangle size={ICON_SIZE.md} style={{ flexShrink: 0, marginTop: 1 }} />
                  <span>
                    {t('settings:biometric_weak_unsupported')}
                  </span>
                </div>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                  }}
                >
                  <span
                    style={{
                      fontSize: 'var(--text-body)',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 6,
                    }}
                  >
                    <ScanFace size={ICON_SIZE.md} style={{ color: 'var(--text-tertiary)' }} />
                    {t('settings:biometric_toggle_label', { type: weakType })}
                  </span>
                  <div style={{ opacity: 0.45 }}>
                    <ToggleSwitch
                      checked={!!bioAvailable.weakConfigured}
                      onChange={() => handleBioToggle('weak')}
                      disabled={!bioAvailable.weakConfigured}
                    />
                  </div>
                </div>
              </>
            )}
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
              {bioMode === 'weak' ? (
                <ScanFace size={ICON_SIZE.xl} />
              ) : (
                <Fingerprint size={ICON_SIZE.xl} />
              )}
              {bioAction === 'enable'
                ? t('settings:biometric_enable_prompt', { type: modeType })
                : t('settings:biometric_disable_prompt', { type: modeType })}
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
