import { useState, useEffect, useLayoutEffect, useCallback, useRef } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { onRequestSessionChange } from '@/lib/sessionRequests';
import { logger } from '@/lib/logger';

type VerificationMethod = 'password' | 'pin' | 'touchId' | 'faceId' | 'windowsHello';

/**
 * P013/5: 工作区敏感操作密码/生物识别守卫。
 * 详情面板与历史查看器共用——通过 passwordVerify() 打开验证对话框，
 * 验证结果在 Hook 内按请求代次回传；账户/会话切换时取消待决验证。
 */
export function useWorkspacePasswordGuard(accountId: string | undefined) {
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<
    ((result: { ok: boolean; method: VerificationMethod }) => void) | null
  >(null);
  const verificationSequence = useRef(0);
  const [verificationId, setVerificationId] = useState(0);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  const handlePwDialogClose = useCallback(() => {
    verificationSequence.current += 1;
    pwResolveRef.current?.({ ok: false, method: 'password' });
    pwResolveRef.current = null;
    setShowPwDialog(false);
  }, []);

  useLayoutEffect(() => {
    handlePwDialogClose();
    const unsubscribe = onRequestSessionChange(handlePwDialogClose);
    return () => {
      unsubscribe();
      handlePwDialogClose();
    };
  }, [accountId, handlePwDialogClose]);

  useEffect(() => {
    let active = true;
    setBioAvailable({ available: false });
    setPasswordHint(null);
    if (!accountId) return;
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      { accountId },
    )
      .then((r) => {
        if (active)
          setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType });
      })
      .catch((err) => logger.warn('[Workspace] Biometric check failed:', err));
    invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
      .then((accounts) => {
        if (!active) return;
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      })
      .catch(() => {
        /* ignore */
      });
    return () => {
      active = false;
    };
  }, [accountId]);

  const passwordVerify = useCallback(async (): Promise<{
    ok: boolean;
    method: VerificationMethod;
  }> => {
    return new Promise((resolve) => {
      pwResolveRef.current?.({ ok: false, method: 'password' });
      pwResolveRef.current = resolve;
      setVerificationId(++verificationSequence.current);
      setShowPwDialog(true);
    });
  }, []);

  const verifyVaultPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        await invoke('unlock_with_password', { accountId: accountId, password });
        return true;
      } catch (err) {
        // P124: 密码错误与后端异常可区分——后端对错误密码返回 Err("Invalid password")，
        // 返回 false（对话框显示「密码不正确」）；其余为真实后端异常，抛出保留细节
        // （对话框 catch 走 onError toast），不再无差别当作密码错误。
        const msg =
          typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
        if (/invalid password|incorrect password|密码错误|密码不正确/i.test(msg)) {
          return false;
        }
        logger.warn('[Workspace] Vault unlock failed:', err);
        throw err;
      }
    },
    [accountId],
  );

  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    const resolve = pwResolveRef.current;
    const id = verificationSequence.current;
    if (!accountId || !resolve) return false;
    try {
      await invoke('biometric_unlock', {
        accountId: accountId,
        location: 'critical_data_access',
        action: 'unlock',
        biometryType: bioAvailable.biometryType,
      });
      const method: VerificationMethod =
        bioAvailable.biometryType === 'faceId' || bioAvailable.biometryType === 'windowsHello'
          ? bioAvailable.biometryType
          : 'touchId';
      if (pwResolveRef.current !== resolve || verificationSequence.current !== id) return false;
      resolve({ ok: true, method });
      pwResolveRef.current = null;
      return true;
    } catch (err) {
      // P124: 记录失败细节（用户取消 vs 后端异常在 UI 上保持静默停留，但日志不再丢失）
      logger.warn('[Workspace] Biometric unlock failed:', err);
      return false;
    }
  }, [accountId, bioAvailable.biometryType]);

  const handlePwDialogVerify = useCallback(
    async (password: string): Promise<boolean> => {
      const resolve = pwResolveRef.current;
      const id = verificationSequence.current;
      if (!resolve) return false;
      let ok: boolean;
      try {
        ok = await verifyVaultPassword(password);
      } catch (error) {
        if (pwResolveRef.current !== resolve || verificationSequence.current !== id) return false;
        throw error;
      }
      if (pwResolveRef.current !== resolve || verificationSequence.current !== id) return false;
      if (ok) {
        resolve({ ok: true, method: 'password' });
        pwResolveRef.current = null;
      }
      return ok;
    },
    [verifyVaultPassword],
  );

  const handlePwDialogPinSuccess = useCallback(() => {
    if (verificationId !== verificationSequence.current) return;
    pwResolveRef.current?.({ ok: true, method: 'pin' });
    pwResolveRef.current = null;
    setShowPwDialog(false);
  }, [verificationId]);

  return {
    showPwDialog,
    bioAvailable,
    passwordHint,
    passwordVerify,
    handlePwDialogClose,
    handlePwDialogVerify,
    handlePwDialogPinSuccess,
    handleBiometricUnlock,
  };
}
