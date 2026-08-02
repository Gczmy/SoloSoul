import { create } from 'zustand';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { AccountInfo } from '@/lib/ipc';
import { logger } from '@/lib/logger';

export const LAST_ACCOUNT_KEY = 'solosoul_last_account_id';

/** 持久化最近一次成功登录的账户 ID，供登录页默认选中 */
export function saveLastAccountId(accountId: string) {
  try {
    localStorage.setItem(LAST_ACCOUNT_KEY, accountId);
  } catch {
    // localStorage 不可用时静默降级
  }
}

interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  currentAccount: AccountInfo | null;
  accounts: AccountInfo[];
  error: string | null;
  hasAccount: boolean | null;
  backendError: boolean;

  checkHasAccount: () => Promise<void>;
  listAccounts: () => Promise<void>;
  refreshCurrentAccount: () => Promise<void>;
  bootstrap: (
    name: string,
    password: string,
    locale: string,
    passwordHint?: string,
  ) => Promise<void>;
  login: (accountId: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  /** 锁定 Vault（收敛自 vaultStore.lock）。无论后端调用成功与否都重置认证状态。 */
  lock: () => Promise<void>;
  /** 完成解锁（P015：收敛 LoginPage PIN/生物识别两条直改 setState 的路径）。
   *  accounts 可选——PIN 解锁后端直接返回账户信息，无需二次拉取；
   *  生物识别路径已拉取账户列表，可传入一并更新。 */
  completeUnlock: (account: AccountInfo, accounts?: AccountInfo[]) => void;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  isAuthenticated: false,
  isLoading: false,
  currentAccount: null,
  accounts: [],
  error: null,
  hasAccount: null,
  backendError: false,    checkHasAccount: async () => {
    try {
      const result = await invoke<boolean>('check_has_account');
      set({ hasAccount: result, backendError: false });
    } catch (err) {
      // Backend unavailable — don't jump to bootstrap
      // hasAccount stays null (unknown), BootstrapGuard will wait
      logger.error('[authStore] check_has_account failed:', err);
      set({ backendError: true });
    }
  },

  listAccounts: async () => {
    try {
      const accounts = await invoke<AccountInfo[]>('vault_list_accounts');
      const currentId = get().currentAccount?.id;
      const refreshed = currentId ? accounts.find((a) => a.id === currentId) : null;
      set({
        accounts,
        currentAccount: refreshed || get().currentAccount,
        hasAccount: accounts.length > 0,
        backendError: false,
      });
    } catch (err) {
      // Surface the error so the user can report it; vault may be locked.
      set({
        error: String(err),
        backendError: true,
      });
    }
  },

  refreshCurrentAccount: async () => {
    try {
      const accounts = await invoke<AccountInfo[]>('vault_list_accounts');
      const currentId = get().currentAccount?.id;
      const refreshed = currentId ? accounts.find((a) => a.id === currentId) : null;
      if (refreshed) {
        set({ currentAccount: refreshed, accounts });
      }
    } catch (err) {
      // P227: 刷新失败静默降级可接受，但需留痕便于排查。
      logger.warn('[authStore] refreshCurrentAccount failed:', err);
    }
  },

  bootstrap: async (name, password, locale, passwordHint) => {
    set({ isLoading: true, error: null });
    try {
      const account = await invoke<AccountInfo>('bootstrap', {
        accountName: name,
        password,
        locale,
        passwordHint: passwordHint,
      });
      saveLastAccountId(account.id);
      set({
        isAuthenticated: true,
        currentAccount: account,
        accounts: [account],
        hasAccount: true,
        isLoading: false,
        backendError: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  login: async (accountId, password) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<void>('login', { accountId: accountId, password });
      // Try to refresh account list, but do not fail authentication if the
      // refresh request errors (e.g. transient backend lock contention).
      let accounts: AccountInfo[] = [];
      try {
        accounts = (await invoke<AccountInfo[]>('vault_list_accounts')) || [];
      } catch (err) {
        // Keep authentication state even if the account-list refresh fails.
        // P227: 登录后的账户列表刷新失败属可接受降级，但留痕便于排查。
        logger.warn('[authStore] account list refresh after login failed:', err);
      }
      const account = accounts.find((a) => a.id === accountId) || {
        id: accountId,
        name: accountId,
      };
      saveLastAccountId(account.id);
      // 记录解锁完成时刻供 T2 性能基线使用
      (window as typeof window & { __SOLOSOUL_UNLOCK_TIME?: number }).__SOLOSOUL_UNLOCK_TIME =
        performance.now();

      set({
        isAuthenticated: true,
        currentAccount: account,
        accounts,
        isLoading: false,
      });

      // 解锁后延迟检查备份提醒，避免启动时立即弹权限/通知
      setTimeout(() => {
        import('@/lib/notification')
          .then((m) => m.checkBackupReminder())
          .catch((err) => logger.warn('[authStore] backup reminder check failed:', err));
      }, 2000);
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  logout: async () => {
    // P014: 状态重置必须无条件执行。先清前端认证状态，再尽力通知后端——
    // 即使后端 invoke 失败（如 Vault 已锁定或后端正在重新初始化），
    // 也不会出现半认证僵尸态（AuthGuard 继续放行受保护路由）。
    set({
      isAuthenticated: false,
      currentAccount: null,
      accounts: [],
      // hasAccount 保持为 null（未知），由 BootstrapGuard 重新调用
      // checkHasAccount 确认后端账户状态。
      hasAccount: null,
    });
    try {
      await invoke<void>('logout');
    } catch (err) {
      logger.warn('[authStore] logout invoke failed (auth state already reset):', err);
    }
  },

  lock: async () => {
    // P015: 锁定收敛为 authStore action（替代 vaultStore.lock）。
    // 先清前端认证状态，再尽力通知后端。
    // 锁定不改变账户存在性，因此保留 hasAccount/accounts——
    // 否则 vault-locked 事件丢失（本修复针对的场景）时 /login 会卡在
    // hasAccount===null 的 Connecting... 分支上。
    set({
      isAuthenticated: false,
      currentAccount: null,
    });
    try {
      await invoke<void>('lock');
    } catch (err) {
      logger.warn('[authStore] lock invoke failed (auth state already reset):', err);
    }
  },

  completeUnlock: (account, accounts) => {
    // P015: 统一解锁状态写入路径（PIN/生物识别不再各自直改 setState）。
    // accounts 可选：未提供时保留现有列表，避免 PIN 路径额外拉取。
    set((state) => ({
      isAuthenticated: true,
      currentAccount: account,
      accounts: accounts ?? state.accounts,
      error: null,
      isLoading: false,
    }));
  },

  clearError: () => set({ error: null }),
}));
