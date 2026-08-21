import { useEffect, useRef, useState, useCallback } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { logger } from '@/lib/logger';
import { useRevealState } from '@/hooks/useRevealState';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import type { ObjectData, ObjectSummary } from '@/stores/objectStore';

export interface UseObjectDetailVerificationOptions {
  /** 当前账户 id（探测生物识别可用性与密码提示）。 */
  accountId?: string;
  /** 当前对象（访问日志取 name/id/typeId）；可为 null（加载中）。 */
  obj: ObjectData | ObjectSummary | null;
  /** 集合名称解析（父 hook 的 useCallback 传入，身份稳定）。 */
  resolveCollectionLabelLocal: (typeId: string) => string;
}

/**
 * 关键数据验证域（W001-④ 拆分：数据 hook）。
 * 密码验证对话框联动（pwResolveRef 收敛）、生物识别解锁、关键字段揭示与访问日志、
 * 生物识别可用性/密码提示探测均收敛于此；父 hook 仅组合与透传。
 * 含 useRevealState——揭示状态与验证流程同属关键数据访问语义，随迁保持内聚。
 */
export function useObjectDetailVerification({
  accountId,
  obj,
  resolveCollectionLabelLocal,
}: UseObjectDetailVerificationOptions) {
  const { maskValue, isRevealed, reveal, revealRemainingMs } = useRevealState();
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<
    | ((result: {
        ok: boolean;
        method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';
      }) => void)
    | null
  >(null);
  const pendingRevealRef = useRef<{ fieldId: string; fieldName: string } | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  useEffect(() => {
    if (!accountId) return;
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      {
        accountId: accountId,
      },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch((err) => logger.warn('[ObjectDetail] Biometric availability check failed:', err));
    invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
      .then((accounts) => {
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      })
      .catch((err) => logger.warn('[ObjectDetail] Load password hint failed:', err));
  }, [accountId]);

  const unlockVaultWithPassword = useCallback(
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
        logger.warn('[ObjectDetail] Vault unlock failed:', err);
        throw err;
      }
    },
    [accountId],
  );

  const passwordVerify = useCallback(async (): Promise<{
    ok: boolean;
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';
  }> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  const writeCriticalAccessLog = useCallback(
    async (method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin') => {
      if (!accountId || !obj || !pendingRevealRef.current) return;
      const actionType =
        method === 'password'
          ? 'critical_field_login'
          : method === 'pin'
            ? 'critical_field_pin'
            : method === 'touchId'
              ? 'critical_field_touch_id'
              : method === 'windowsHello'
                ? 'critical_field_windows_hello'
                : 'critical_field_face_id';
      const entityType = method === 'password' || method === 'pin' ? 'auth' : 'biometric';
      const details = `objectName=${obj.name} page=${resolveCollectionLabelLocal(obj.typeId)} fieldName=${pendingRevealRef.current.fieldName}`;
      try {
        await invoke('log_write', {
          request: {
            actionType,
            entityType,
            entityId: obj.id,
            entityName: null,
            details,
          },
        });
      } catch {
        // best effort
      }
    },
    [accountId, obj, resolveCollectionLabelLocal],
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
      const method =
        (bioAvailable.biometryType as 'touchId' | 'faceId' | 'windowsHello') || 'touchId';
      pwResolveRef.current?.({ ok: true, method });
      return true;
    } catch (err) {
      // P124: 记录失败细节（用户取消 vs 后端异常在 UI 上保持静默停留，但日志不再丢失）
      logger.warn('[ObjectDetail] Biometric unlock failed:', err);
      return false;
    }
  }, [accountId, bioAvailable.biometryType]);

  const handleRevealField = useCallback(
    async (fieldId: string, sens: SensitivityLevel, fieldName: string) => {
      if (sens === 'critical') {
        pendingRevealRef.current = { fieldId, fieldName };
        const result = await passwordVerify();
        if (result.ok) {
          reveal(fieldId);
          await writeCriticalAccessLog(result.method);
        }
      } else {
        reveal(fieldId);
      }
    },
    [passwordVerify, reveal, writeCriticalAccessLog],
  );

  // 密码验证对话框的联动 handler（验证成功/取消/ PIN 成功），收敛 pwResolveRef 细节
  const handlePwDialogClose = useCallback(() => {
    setShowPwDialog(false);
    pwResolveRef.current?.({ ok: false, method: 'password' });
  }, []);

  const handlePwDialogVerify = useCallback(
    async (password: string) => {
      const ok = await unlockVaultWithPassword(password);
      if (ok) pwResolveRef.current?.({ ok: true, method: 'password' });
      return ok;
    },
    [unlockVaultWithPassword],
  );

  const handlePwDialogPinSuccess = useCallback(() => {
    pwResolveRef.current?.({ ok: true, method: 'pin' });
    setShowPwDialog(false);
  }, []);

  return {
    // 揭示状态（useRevealState 随迁——与验证流程同属关键数据访问语义）
    isRevealed,
    revealRemainingMs,
    maskValue,
    handleRevealField,
    // 关键数据验证
    passwordVerify,
    showPwDialog,
    handlePwDialogClose,
    handlePwDialogVerify,
    handlePwDialogPinSuccess,
    passwordHint,
    bioAvailable,
    handleBiometricUnlock,
  };
}
