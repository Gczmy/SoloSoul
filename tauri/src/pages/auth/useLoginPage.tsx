import { useState, useEffect, useRef } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore, LAST_ACCOUNT_KEY } from '@/stores/authStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import type { AccountInfo } from '@/lib/ipc';

import { useLoginUnlockFlows } from './useLoginUnlockFlows';
import { useLoginIconBar } from './useLoginIconBar';

/** P038: 受支持的生物识别类型白名单（显示名由 LoginBiometricView 的查表负责） */
const BIOMETRIC_INFO: Record<string, string> = {
  faceId: 'faceId',
  touchId: 'touchId',
  windowsHello: 'windowsHello',
};

/** 模块级缓存 — 跨组件卸载持久化，避免锁定后重新挂载时闪烁 */
let _cachedLoginMethod: 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password' | null = null;

export type LoginMethod = 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password';

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
        const info = r.biometryType ? BIOMETRIC_INFO[r.biometryType] : undefined;
        if (info) {
          setBiometryTypeRaw(info);
        }
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
  // 从模块缓存初始化，避免锁定后重新挂载时闪烁
  const [loginMethod, setLoginMethod] = useState<LoginMethod | null>(_cachedLoginMethod);

  // 跨卸载持久化 — 锁定再登录后直接显示最后使用的方法
  useEffect(() => {
    if (loginMethod) _cachedLoginMethod = loginMethod;
  }, [loginMethod]);

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

    // Check biometric
    // lockout 场景：系统因失败次数过多临时锁定生物识别（Android canAuthenticate 返回
    // ERROR_LOCKOUT），此时后端 available 会变 false，但凭证仍已配置（configured=true）。
    // 指纹项应继续显示，仅在解锁时提示"系统指纹识别未恢复"，而不是消失或显示"不支持"。
    invoke<{
      available: boolean;
      configured: boolean;
      biometryType?: string;
      lockout?: boolean;
    }>('biometric_check_availability', { accountId: selectedAccountId })
      .then((r) => {
        if (controller.signal.aborted) return;
        // 已配置凭证且（设备可用或系统临时锁定）→ 保留指纹项。
        // lockout 以 !!r.lockout 为准：即使 available 与 lockout 同时成立
        // （Android 插件 status() 在锁定期间可能仍报可用），也正确显示警告。
        if (r.configured && (r.available || r.lockout)) {
          setBioAvailable(true);
          setBioLockout(!!r.lockout);
          const info = r.biometryType ? BIOMETRIC_INFO[r.biometryType] : undefined;
          if (info) {
            setBiometryTypeRaw(info);
          }
        } else {
          setBioAvailable(false);
          setBioLockout(false);
        }
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setBioAvailable(false);
      })
      .finally(() => {
        if (!controller.signal.aborted) setBioChecked(true);
      });

    // Check PIN
    invoke<{ configured: boolean; locked: boolean }>('pin_check_availability', {
      accountId: selectedAccountId,
    })
      .then((r) => {
        if (controller.signal.aborted) return;
        setPinAvailable(r.configured && !r.locked);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setPinAvailable(false);
      })
      .finally(() => {
        if (!controller.signal.aborted) setPinChecked(true);
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
