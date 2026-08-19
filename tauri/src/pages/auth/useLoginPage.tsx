import { useState, useEffect, useRef } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore, LAST_ACCOUNT_KEY } from '@/stores/authStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import type { AccountInfo } from '@/lib/ipc';

import {
  readCachedLoginMethod,
  writeCachedLoginMethod,
  type LoginMethod,
} from '@/lib/loginMethodCache';
import {
  normalizeBiometryType,
  preflightLoginAvailability,
} from '@/lib/loginAvailabilityPreflight';

import { useLoginUnlockFlows } from './useLoginUnlockFlows';
import { useLoginIconBar } from './useLoginIconBar';

export type { LoginMethod } from '@/lib/loginMethodCache';

/**
 * 登录页全部编排逻辑（P046 拆分：数据 hook；W001-② 再拆后为组合层）。
 * 账户选择、生物识别/PIN 可用性探测、解锁方式优先级与模块缓存、底部图标栏组合
 * 保留于此；三种解锁 handler 收敛于 useLoginUnlockFlows，图标栏状态收敛于
 * useLoginIconBar。
 */
export function useLoginPage() {
  useApplyThemeFromSettings();
  const navigate = useNavigate();
  // P022: useShallow 字段级选择——避免 store 无关字段（error/backendError）翻转时整页重渲染
  const { checkHasAccount, listAccounts, hasAccount, isAuthenticated, isLoading, accounts } =
    useAuthStore(
      useShallow((s) => ({
        checkHasAccount: s.checkHasAccount,
        listAccounts: s.listAccounts,
        hasAccount: s.hasAccount,
        isAuthenticated: s.isAuthenticated,
        isLoading: s.isLoading,
        accounts: s.accounts,
      })),
    );
  const [searchParams] = useSearchParams();
  const fromExisting = searchParams.get('fromExisting') === 'true';
  const [selectedAccountId, setSelectedAccountId] = useState('');
  const selectedAccount = accounts.find((a) => a.id === selectedAccountId);

  // Biometric state
  const [bioAvailable, setBioAvailable] = useState(false);
  const [biometryTypeRaw, setBiometryTypeRaw] = useState('touchId');
  const [bioChecked, setBioChecked] = useState(false);
  // 系统生物识别因失败次数过多被临时锁定（Android）：指纹项仍显示，但点击时提示警告
  const [bioLockout, setBioLockout] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  // PIN state
  const [pinAvailable, setPinAvailable] = useState(false);
  const [pinChecked, setPinChecked] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);

  // 移动端启动性能基线：登录页首个输入框获焦时记录 T1（MOB-P1-07）
  // 注意 T1 从 __SOLOSOUL_APP_START_TIME 开始算，记录首个输入框 focus 时刻。
  const t1FiredRef = useRef(false);

  useEffect(() => {
    // Defensive load: fetch the account list directly in case Vite HMR keeps
    // authStore.listAccounts pointing at a stale command name.
    invoke<AccountInfo[]>('vault_list_accounts')
      .then((parsed) => {
        useAuthStore.setState({
          accounts: parsed,
          hasAccount: parsed.length > 0,
        });
      })
      .catch(() => {
        // Fall through to the store-level loader below.
      });
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    // 到达登录页即撤掉原生锁屏遮盖层（Android；其他平台为 no-op）。
    // 覆盖冷启动路径：进程被杀后持久化标记触发遮盖，但 JS 事件可能丢失，
    // 冷启动后 Vault 内存密钥已不存在，必然落在登录页，挂载即撤遮盖。
    invoke('dismiss_lock_mask').catch(() => {});
    checkHasAccount().then(() => {
      if (!ctrl.signal.aborted) listAccounts();
    });
    // Probe device biometry type early (do not set bioAvailable here — configured
    // status is account-specific and decided in the selectedAccountId effect).
    invoke<{ available: boolean; biometryType?: string }>('biometric_check_availability', {
      accountId: '',
    })
      .then((r) => {
        if (ctrl.signal.aborted) return;
        // 设备级提示：账户级探测（preflight）会再次校正
        setBiometryTypeRaw(normalizeBiometryType(r.biometryType));
      })
      .catch(() => {
        // Ignore: account-specific check will handle availability.
      });
    // 每次回到前台时撤掉原生锁屏遮盖层：
    // Kotlin LockStatePlugin.onResume() 在进程被杀恢复后可能重新挂遮罩，
    // 而 useAutoLock 的 screen-locked 监听器在 isAuthenticated 变为 false
    // 时已被清理，导致无人调 dismiss_lock_mask。此兜底确保无论何时回到前台都撤除。
    const onVisibleDismissMask = () => {
      if (document.visibilityState === 'visible') {
        invoke('dismiss_lock_mask').catch(() => {});
      }
    };
    document.addEventListener('visibilitychange', onVisibleDismissMask);
    return () => {
      document.removeEventListener('visibilitychange', onVisibleDismissMask);
      ctrl.abort();
    };
  }, [checkHasAccount, listAccounts]);

  useEffect(() => {
    if (hasAccount === false) navigate('/bootstrap');
    if (isAuthenticated) navigate('/');
  }, [hasAccount, isAuthenticated, navigate]);

  // Auto-select last logged-in account (fall back to first account)
  useEffect(() => {
    if (accounts.length > 0 && !selectedAccountId) {
      let lastId = '';
      try {
        lastId = localStorage.getItem(LAST_ACCOUNT_KEY) || '';
      } catch {
        lastId = '';
      }
      const target = accounts.find((a) => a.id === lastId) || accounts[0];
      setSelectedAccountId(target.id);
    }
  }, [accounts, selectedAccountId]);

  // Priority-based login method selection
  // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
  // 方案 A：从 localStorage 按账户同步恢复上次登录方式——冷启动首帧即正确方法，
  // 消灭「先显示主密码再跳指纹」闪屏；可用性探测完成后仍会校正过期缓存。
  const [loginMethod, setLoginMethod] = useState<LoginMethod | null>(() => {
    let lastId = '';
    try {
      lastId = localStorage.getItem(LAST_ACCOUNT_KEY) || '';
    } catch {
      // localStorage 不可用时保持空串（无缓存 → 探测中占位）
    }
    return readCachedLoginMethod(lastId);
  });

  // 跨卸载持久化（localStorage，按账户隔离）——锁定再登录后直接显示最后使用的方法
  useEffect(() => {
    if (!loginMethod) return;
    const accountId =
      selectedAccountId ||
      (() => {
        try {
          return localStorage.getItem(LAST_ACCOUNT_KEY) || '';
        } catch {
          return '';
        }
      })();
    if (accountId) writeCachedLoginMethod(accountId, loginMethod);
  }, [loginMethod, selectedAccountId]);

  // 三种解锁流程（PIN / 生物识别 / 主密码）+ 各自状态
  // 置于可用性 effect 之前：复位块需经组合层转调其 setter（setter 身份稳定）
  const unlockFlows = useLoginUnlockFlows({
    selectedAccountId,
    fromExisting,
    bioLockout,
    biometryTypeRaw,
    setLoginMethod,
    setBioLockout,
    setPinAvailable,
  });

  // 稳定 setter 解构：供可用性 effect 的复位块与图标栏错误清除复用（useState setter 身份稳定）
  const { setBioError, setPinError, setSubmitError } = unlockFlows;

  // Check biometric and PIN availability for selected account
  useEffect(() => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    if (!selectedAccountId) {
      // 尚未选中账户时保持缓存值，不触发优先级设置 effect，避免覆盖缓存
      return () => controller.abort();
    }

    // 从 SAF 已有账户登录时，旧安装的生物识别/PIN 凭证已失效，强制仅显示主密码
    if (fromExisting) {
      setBioChecked(true);
      setPinChecked(true);
      setBioAvailable(false);
      setPinAvailable(false);
      setBioLockout(false);
      // 直接定为主密码，避免 localStorage 缓存的上次方式短暂闪现
      setLoginMethod('password');
      return () => controller.abort();
    }

    // Reset state when account changes — 不重置 loginMethod，保留缓存值避免闪烁
    setBioChecked(false);
    setPinChecked(false);
    setBioAvailable(false);
    setPinAvailable(false);
    setBioLockout(false);
    // 错误状态（unlockFlows 持有）同样复位——原内联实现同处复位，拆分后经组合层转调
    setBioError(null);
    setPinError(null);
    setSubmitError(null);

    // 指纹/PIN 可用性探测：统一走预探测路径（方案 C）——启动期（main.tsx）可能
    // 已按 LAST_ACCOUNT_KEY 发起同账户探测，这里直接复用结果；未发起则首次发起。
    // 探测完成前保持固定高度占位（方案 B），不再先渲染主密码。
    preflightLoginAvailability(selectedAccountId)
      .then((r) => {
        if (controller.signal.aborted) return;
        setBioAvailable(r.bioAvailable);
        setBioLockout(r.bioLockout);
        setBiometryTypeRaw(r.biometryTypeRaw);
        setPinAvailable(r.pinAvailable);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setBioAvailable(false);
        setPinAvailable(false);
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setBioChecked(true);
          setPinChecked(true);
        }
      });

    return () => controller.abort();
  }, [selectedAccountId, fromExisting, setBioError, setPinError, setSubmitError]);

  // Set login method by priority after both checks complete
  useEffect(() => {
    if (!bioChecked || !pinChecked) return;

    // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
    if (bioAvailable) {
      const raw = biometryTypeRaw;
      if (raw === 'faceId') setLoginMethod('faceId');
      else if (raw === 'touchId') setLoginMethod('touchId');
      else if (raw === 'windowsHello') setLoginMethod('windowsHello');
      else setLoginMethod('password');
    } else if (pinAvailable) {
      setLoginMethod('pin');
    } else {
      setLoginMethod('password');
    }
  }, [bioChecked, pinChecked, bioAvailable, pinAvailable, biometryTypeRaw]);

  // 图标栏：可用解锁方式列表 + 两阶段悬停状态
  // 选择方式时清对应错误（bio 方法清 bioError、PIN 清 pinError，与原内联行为一致）
  const iconBar = useLoginIconBar({
    bioAvailable,
    biometryTypeRaw,
    pinAvailable,
    onSelectMethod: (method) => {
      setLoginMethod(method);
      if (method === 'pin') {
        setPinError(null);
      } else if (method !== 'password') {
        setBioError(null);
      }
    },
  });

  return {
    // account
    accounts,
    selectedAccountId,
    setSelectedAccountId,
    selectedAccount,
    // password input（来自 unlockFlows）
    password: unlockFlows.password,
    setPassword: unlockFlows.setPassword,
    passwordFieldError: unlockFlows.passwordFieldError,
    setPasswordFieldError: unlockFlows.setPasswordFieldError,
    passwordErrorTick: unlockFlows.passwordErrorTick,
    // store-driven
    isLoading,
    // method & availability
    loginMethod,
    // biometric view
    bioLoading: unlockFlows.bioLoading,
    bioLockout,
    bioError: unlockFlows.bioError,
    // pin view
    pinUnlocking: unlockFlows.pinUnlocking,
    pinError: unlockFlows.pinError,
    pinInputKey: unlockFlows.pinInputKey,
    pinInputRef: unlockFlows.pinInputRef,
    // password view extras
    submitError: unlockFlows.submitError,
    // handlers
    handleBiometricUnlock: unlockFlows.handleBiometricUnlock,
    handlePinComplete: unlockFlows.handlePinComplete,
    handleSubmit: unlockFlows.handleSubmit,
    // icon bar
    iconMethods: iconBar.iconMethods,
    hoveredIcon: iconBar.hoveredIcon,
    committedIcon: iconBar.committedIcon,
    handleIconEnter: iconBar.handleIconEnter,
    handleIconLeave: iconBar.handleIconLeave,
    handleIconClick: iconBar.handleIconClick,
    // links & recovery
    recoveryOpen,
    setRecoveryOpen,
    listAccounts,
    navigate,
    // performance probe
    t1FiredRef,
  };
}
