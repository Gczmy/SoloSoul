import { useState, useEffect, useCallback, useRef } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { logger } from '@/lib/logger';

/**
 * P013/5: 工作区敏感操作密码/生物识别守卫。
 * 详情面板与历史查看器共用——通过 passwordVerify() 打开验证对话框，
 * 结果经 pwResolveRef 回传，UI 侧用 showPwDialog / setShowPwDialog 控制。
 */
export function useWorkspacePasswordGuard(accountId: string | undefined) {
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<
    ((result: { ok: boolean; method: 'password' | 'touchId' | 'faceId' }) => void) | null
  >(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      { accountId: accountId || '' },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch((err) => logger.warn('[Workspace] Biometric check failed:', err));
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

  const passwordVerify = useCallback(async (): Promise<{
    ok: boolean;
    method: 'password' | 'touchId' | 'faceId';
  }> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
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
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', {
        accountId: accountId,
        location: 'critical_data_access',
        action: 'unlock',
        biometryType: bioAvailable.biometryType,
      });
      const method = (bioAvailable.biometryType as 'touchId' | 'faceId') || 'touchId';
      pwResolveRef.current?.({ ok: true, method });
      return true;
    } catch (err) {
      // P124: 记录失败细节（用户取消 vs 后端异常在 UI 上保持静默停留，但日志不再丢失）
      logger.warn('[Workspace] Biometric unlock failed:', err);
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
