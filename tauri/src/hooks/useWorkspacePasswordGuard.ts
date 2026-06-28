import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';

interface PasswordVerifyResult {
  ok: boolean;
  method: 'password' | 'touchId' | 'faceId';
}

interface UseWorkspacePasswordGuardReturn {
  showPwDialog: boolean;
  setShowPwDialog: (v: boolean) => void;
  pwResolveRef: React.MutableRefObject<((result: PasswordVerifyResult) => void) | null>;
  bioAvailable: { available: boolean; biometryType?: string };
  passwordHint: string | null;
  passwordVerify: () => Promise<PasswordVerifyResult>;
  verifyVaultPassword: (password: string) => Promise<boolean>;
  handleBiometricUnlock: () => Promise<boolean>;
}

export function useWorkspacePasswordGuard(): UseWorkspacePasswordGuardReturn {
  const accountId = useAuthStore((s) => s.currentAccount?.id);

  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<((result: PasswordVerifyResult) => void) | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  // Check biometric availability on mount + load password hint
  useEffect(() => {
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      { accountId: accountId || '' },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch((err) => console.warn('[useWorkspacePasswordGuard] Biometric check failed:', err));
    if (accountId) {
      invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
        .then((accounts) => {
          const acc = accounts.find((a) => a.id === accountId);
          setPasswordHint(acc?.passwordHint || null);
        })
        .catch(() => {
          /* ignore */
        });
    }
  }, [accountId]);

  const passwordVerify = useCallback(async (): Promise<PasswordVerifyResult> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  const verifyVaultPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        await invoke('unlock_with_password', { accountId, password });
        return true;
      } catch {
        return false;
      }
    },
    [accountId],
  );

  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', {
        accountId,
        location: 'critical_data_access',
        action: 'unlock',
        biometryType: bioAvailable.biometryType,
      });
      const method = (bioAvailable.biometryType as 'touchId' | 'faceId') || 'touchId';
      pwResolveRef.current?.({ ok: true, method });
      return true;
    } catch {
      return false;
    }
  }, [accountId, bioAvailable.biometryType]);

  return {
    showPwDialog,
    setShowPwDialog,
    pwResolveRef,
    bioAvailable,
    passwordHint,
    passwordVerify,
    verifyVaultPassword,
    handleBiometricUnlock,
  };
}
