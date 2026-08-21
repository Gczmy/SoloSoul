import { useCallback, useEffect, useRef, useState } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { logger } from '@/lib/logger';
import { useRevealState } from '@/hooks/useRevealState';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import type { FieldSuggestion } from './FieldSuggestions';

/** 推荐条目稳定 ID（useRevealState 的 fieldId 键；跨对象+字段唯一）。 */
export function suggestionItemId(item: FieldSuggestion): string {
  return `${item.objectId}::${item.fieldKey}`;
}

type UnlockMethod = 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';

export interface UseSuggestionRevealResult {
  /** 条目当前是否处于揭示态（1 分钟 TTL，与 §7 揭示语义一致）。 */
  isRevealed: (itemId: string) => boolean;
  /**
   * 点击推荐条目：
   * - public / internal：无操作（本就明文展示）；
   * - sensitive：切换揭示/隐藏；
   * - critical：弹出主密码验证框，验证成功（密码/PIN/生物识别）后揭示并写访问日志。
   */
  handleItemClick: (item: FieldSuggestion) => void;
  showPwDialog: boolean;
  handlePwDialogClose: () => void;
  handlePwDialogVerify: (password: string) => Promise<boolean>;
  handlePwDialogPinSuccess: () => void;
  passwordHint: string | null;
  bioAvailable: { available: boolean; biometryType?: string };
  handleBiometricUnlock: () => Promise<boolean>;
}

/**
 * 字段推荐的揭示域：与对象详情（useObjectDetailVerification）同款语义——
 * sensitive 点击直接揭示（1 分钟 TTL），critical 先弹主密码验证框（支持
 * 密码/PIN/生物识别），验证成功揭示并写 critical_field_* 访问日志（best
 * effort）。public / internal 明文展示，点击无操作。
 */
export function useSuggestionReveal(accountId?: string): UseSuggestionRevealResult {
  const { isRevealed, reveal, hide } = useRevealState();
  const [showPwDialog, setShowPwDialog] = useState(false);
  const resolveRef = useRef<((ok: boolean, method?: UnlockMethod) => void) | null>(null);
  const pendingItemRef = useRef<FieldSuggestion | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  useEffect(() => {
    if (!accountId) return;
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      { accountId },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch((err) => logger.warn('[FieldSuggestions] Biometric availability check failed:', err));
    invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
      .then((accounts) => {
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      })
      .catch((err) => logger.warn('[FieldSuggestions] Load password hint failed:', err));
  }, [accountId]);

  const unlockVaultWithPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        await invoke('unlock_with_password', { accountId, password });
        return true;
      } catch (err) {
        // 与对象详情一致：密码错误返回 false（对话框显示「密码不正确」），
        // 其余为真实后端异常，抛出保留细节。
        const msg =
          typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
        if (/invalid password|incorrect password|密码错误|密码不正确/i.test(msg)) {
          return false;
        }
        logger.warn('[FieldSuggestions] Vault unlock failed:', err);
        throw err;
      }
    },
    [accountId],
  );

  const writeCriticalAccessLog = useCallback(
    async (method: UnlockMethod) => {
      const item = pendingItemRef.current;
      if (!accountId || !item) return;
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
      try {
        await invoke('log_write', {
          request: {
            actionType,
            entityType,
            entityId: item.objectId,
            entityName: item.objectName,
            details: `fieldName=${item.fieldName} source=field_suggestion`,
          },
        });
      } catch {
        // best effort
      }
    },
    [accountId],
  );

  const handleItemClick = useCallback(
    (item: FieldSuggestion) => {
      const id = suggestionItemId(item);
      const level = (item.sensitivityLevel as SensitivityLevel) || 'internal';
      if (level === 'public' || level === 'internal') return;
      if (isRevealed(id)) {
        hide(id);
        return;
      }
      if (level === 'critical') {
        pendingItemRef.current = item;
        setShowPwDialog(true);
        resolveRef.current = (ok, method) => {
          if (ok) {
            reveal(id);
            void writeCriticalAccessLog(method ?? 'password');
          }
        };
        return;
      }
      reveal(id);
    },
    [isRevealed, hide, reveal, writeCriticalAccessLog],
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
      resolveRef.current?.(true, (bioAvailable.biometryType as UnlockMethod) || 'touchId');
      return true;
    } catch (err) {
      logger.warn('[FieldSuggestions] Biometric unlock failed:', err);
      return false;
    }
  }, [accountId, bioAvailable.biometryType]);

  const handlePwDialogClose = useCallback(() => {
    setShowPwDialog(false);
    resolveRef.current?.(false);
  }, []);

  const handlePwDialogVerify = useCallback(
    async (password: string) => {
      const ok = await unlockVaultWithPassword(password);
      if (ok) resolveRef.current?.(true, 'password');
      return ok;
    },
    [unlockVaultWithPassword],
  );

  const handlePwDialogPinSuccess = useCallback(() => {
    resolveRef.current?.(true, 'pin');
    setShowPwDialog(false);
  }, []);

  return {
    isRevealed,
    handleItemClick,
    showPwDialog,
    handlePwDialogClose,
    handlePwDialogVerify,
    handlePwDialogPinSuccess,
    passwordHint,
    bioAvailable,
    handleBiometricUnlock,
  };
}
