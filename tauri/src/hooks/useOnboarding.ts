import { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import {
  Sparkles,
  PlusSquare,
  LayoutTemplate,
  ShieldCheck,
  CheckCircle,
  Folder,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import type { AccountInfo } from '@/lib/ipc';
import { ST_ONBOARDING_SAF_URI } from '@/lib/constants';
import { getPlatform } from '@/lib/platform';
import {
  pickVaultDirectory,
  initVaultDirectory,
  VaultDirPickLostError,
} from '@/lib/vaultDirectory';

export interface OnboardingDialogProps {
  onComplete: () => void;
  onSkip: () => void;
}

export type OnboardingStepKey =
  | 'welcome'
  | 'vault_directory'
  | 'create_object'
  | 'templates'
  | 'security'
  | 'finish';

export interface OnboardingStepDef {
  key: OnboardingStepKey;
  icon: LucideIcon;
}

const baseSteps: readonly OnboardingStepDef[] = [
  { key: 'welcome', icon: Sparkles },
  { key: 'vault_directory', icon: Folder },
  { key: 'create_object', icon: PlusSquare },
  { key: 'templates', icon: LayoutTemplate },
  { key: 'security', icon: ShieldCheck },
  { key: 'finish', icon: CheckCircle },
];

/** SAF 同步进度阶段：idle（未同步）/ syncing（同步中）/ done（同步完成） */
export type SyncPhase = 'idle' | 'syncing' | 'done';

/** OnboardingDialog 的完整状态机与业务逻辑。 */
export function useOnboarding({ onComplete, onSkip: _onSkip }: OnboardingDialogProps) {
  const { t } = useTranslation('common');
  const [step, setStep] = useState(0);
  const [platformName, setPlatformName] = useState<string>('');
  const [vaultDirActing, setVaultDirActing] = useState(false);
  const [vaultDirError, setVaultDirError] = useState<string | null>(null);
  // 外部目录（SAF）选择后先显示路径，等用户手动点击"下一步"再前进。
  // 从 localStorage 恢复可解决 Android 因系统 SAF 选择器导致 Activity 重建后状态丢失的问题。
  const [selectedSafUri, setSelectedSafUri] = useState<string | null>(() => {
    try {
      return localStorage.getItem(ST_ONBOARDING_SAF_URI);
    } catch {
      return null;
    }
  });

  // 将选中的 SAF URI 同步到 localStorage，以便 Android Activity 重建后恢复
  useEffect(() => {
    try {
      if (selectedSafUri) {
        localStorage.setItem(ST_ONBOARDING_SAF_URI, selectedSafUri);
      } else {
        localStorage.removeItem(ST_ONBOARDING_SAF_URI);
      }
    } catch {
      // 某些隐私模式下 localStorage 不可用，忽略错误
    }
  }, [selectedSafUri]);

  const [syncPhase, setSyncPhase] = useState<SyncPhase>('idle');
  const [syncFileName, setSyncFileName] = useState<string>('');
  const [syncFileCount, setSyncFileCount] = useState(0);
  const syncDoneTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const unlistenSync = useRef<{ unregister: () => void } | null>(null);
  // 选择目录后检测到的已有账户列表与决策界面状态
  const [foundAccounts, setFoundAccounts] = useState<AccountInfo[]>([]);
  const [foundAccountCount, setFoundAccountCount] = useState(0);
  const [showAccountDecision, setShowAccountDecision] = useState(false);
  // 完成引导后询问用户是否已有账户在其它设备上
  // （从创建新账户页返回时改走 AccountSourceOverlay 独立浮层，不再复用本向导）
  const [showAccountSourceDecision, setShowAccountSourceDecision] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    getPlatform().then((p) => {
      setPlatformName(p);
    });
  }, []);

  const isAndroid = platformName === 'android';

  // Wait for platform to load before determining steps, avoiding race where
  // the steps array changes length mid-render (vault_directory step only shown on Android).
  const steps: readonly OnboardingStepDef[] =
    platformName === ''
      ? baseSteps
      : isAndroid
        ? baseSteps
        : baseSteps.filter((s) => s.key !== 'vault_directory');

  // 防御性钳制：平台异步解析会改变 steps 长度，在 effect 重新校准前可能出现
  // step 越界（steps[step] 为 undefined）。
  const current = steps[Math.min(step, steps.length - 1)];
  const Icon = current?.icon || Sparkles;
  const isLast = step >= steps.length - 1;

  // 进入 vault_directory 步骤时重置 loading/error 状态，但不重置已选路径，
  // 以便用户返回上一步时仍能看到之前选择的外部目录。
  useEffect(() => {
    if (current?.key === 'vault_directory') {
      setVaultDirError(null);
      setVaultDirActing(false);
      setSyncPhase('idle');
      // 保留之前的账户检测结果（showAccountDecision/foundAccounts），
      // 让用户在返回时仍能看到登录/创建新账户的决策选项。
    }
    return () => {
      if (syncDoneTimer.current) {
        clearTimeout(syncDoneTimer.current);
        syncDoneTimer.current = null;
      }
      // 清理 Kotlin plugin 事件监听
      if (unlistenSync.current) {
        unlistenSync.current.unregister();
        unlistenSync.current = null;
      }
    };
  }, [current?.key]);

  const handleVaultDirPick = useCallback(async () => {
    // 注意：本回调不直接调用 onComplete，决策卡片由用户选择后再决定
    const { pause, resume } = await import('@/stores/autoLockPauseStore').then((m) =>
      m.useAutoLockPauseStore.getState(),
    );
    pause();
    try {
      setVaultDirActing(true);
      setVaultDirError(null);
      setShowAccountDecision(false);
      setFoundAccounts([]);
      const uri = await pickVaultDirectory();
      if (!uri) {
        return;
      }
      // 立即保存 URI 以便用户返回上一步后重新进入时仍能看到选择
      setSelectedSafUri(uri);
      // 开始同步时显示进度条
      setSyncPhase('syncing');
      setSyncFileName('');
      setSyncFileCount(0);

      // 监听 Kotlin 插件同步进度事件
      try {
        const { addPluginListener } = await import('@tauri-apps/api/core');
        if (unlistenSync.current) unlistenSync.current.unregister();
        unlistenSync.current = await addPluginListener<{
          phase: string;
          fileName?: string;
          fileCount?: number;
        }>('attachment-import-plugin', 'sync-progress', (payload) => {
          if (payload.phase === 'syncing') {
            setSyncFileName(payload.fileName ?? '');
            setSyncFileCount(payload.fileCount ?? 0);
          }
        });
      } catch {
        // 桌面端无 Kotlin 插件，静默失败
      }

      const result = await initVaultDirectory(uri);
      if (result.success) {
        if (result.accountCount && result.accountCount > 0) {
          // 已有账户：显示登录/创建决策卡片
          setFoundAccounts(result.accounts ?? []);
          setFoundAccountCount(result.accountCount ?? 0);
          setSyncPhase('idle');
          setShowAccountDecision(true);
          return;
        }
        // 显示 "SAF 同步完成" 3 秒后自动切换到路径显示
        syncDoneTimer.current = setTimeout(() => {
          setSyncPhase('idle');
        }, 3000);
      } else {
        setSyncPhase('idle');
        setVaultDirError(result.message || t('onboarding_vault_dir_set_failed'));
      }
    } catch (e) {
      setSyncPhase('idle');
      setVaultDirError(
        e instanceof VaultDirPickLostError ? t('onboarding_vault_dir_pick_failed') : String(e),
      );
    } finally {
      // 确保在任何路径下都清理 Kotlin 插件事件监听
      if (unlistenSync.current) {
        unlistenSync.current.unregister();
        unlistenSync.current = null;
      }
      resume();
      setVaultDirActing(false);
    }
  }, [t]);

  /** 选择"使用本地目录"：初始化成功后前进到下一步。 */
  const handleLocalDirPick = useCallback(async () => {
    const { pause, resume } = await import('@/stores/autoLockPauseStore').then((m) =>
      m.useAutoLockPauseStore.getState(),
    );
    pause();
    try {
      setVaultDirActing(true);
      const result = await initVaultDirectory(null);
      if (result.success) {
        setStep((s) => s + 1);
      } else {
        setVaultDirError(result.message || t('onboarding_vault_dir_set_failed'));
      }
    } catch (e) {
      setVaultDirError(String(e));
    } finally {
      resume();
      setVaultDirActing(false);
    }
  }, [t]);

  const handleLoginExisting = useCallback(async () => {
    // 刷新全局账户状态，让 AppRoutes 自动路由到 /login
    await useAuthStore.getState().checkHasAccount();
    onComplete();
    navigate('/login?fromExisting=true', { replace: true });
  }, [navigate, onComplete]);

  const handleCreateNewAccount = useCallback(async () => {
    // 直接导航到创建账户页，由 AppRoutes 的特殊 query 处理
    onComplete();
    navigate('/bootstrap?mode=create', { replace: true });
  }, [navigate, onComplete]);

  /** 常规步骤的「下一步/完成」：非最后一步前进；最后一步检查本地账户，无账户则询问账户来源。 */
  const handleFinishClick = useCallback(async () => {
    if (!isLast) {
      setStep((s) => s + 1);
      return;
    }
    // 完成引导后检查是否有已有账户，有则跳转到登录页
    // （SAF 目录同步后账户可能已存在，需从后端重新查询）
    await useAuthStore.getState().checkHasAccount();
    const hasAccount = useAuthStore.getState().hasAccount;
    if (!hasAccount) {
      // 没有本地账户：询问用户是否已有账户在其它设备上
      setShowAccountSourceDecision(true);
      return;
    }
    // 先执行 onComplete() 再 navigate，避免组件卸载后导航丢失
    onComplete();
    navigate('/login', { replace: true });
  }, [isLast, navigate, onComplete]);

  const clearSelectedSafUri = useCallback(() => {
    setSelectedSafUri(null);
  }, []);

  return {
    t,
    step,
    setStep,
    steps,
    current,
    Icon,
    isLast,
    isAndroid,
    vaultDirActing,
    vaultDirError,
    selectedSafUri,
    syncPhase,
    syncFileName,
    syncFileCount,
    foundAccounts,
    foundAccountCount,
    showAccountDecision,
    showAccountSourceDecision,
    recoveryOpen,
    setRecoveryOpen,
    setShowAccountSourceDecision,
    setVaultDirError,
    handleVaultDirPick,
    handleLocalDirPick,
    handleLoginExisting,
    handleCreateNewAccount,
    handleFinishClick,
    clearSelectedSafUri,
  };
}
