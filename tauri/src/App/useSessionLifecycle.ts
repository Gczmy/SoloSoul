import { useEffect } from 'react';
import type { NavigateFunction } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { trackAsyncListener } from '@/lib/asyncListener';
import { dismissStartupScreen } from '@/lib/startupScreen';
import { warmupPrefetchRegistry, resetPrefetchRegistry } from '@/lib/prefetch/warmup';
import { getSystemTheme, applyTheme } from '@/lib/theme';
import { confirmWithPause } from '@/lib/dialog';
import { initLlmNotificationListener } from '@/lib/notification';
import { setGlobalNavigate } from '@/lib/navigation';
import { logger } from '@/lib/logger';
import { useAuthStore } from '@/stores/authStore';
import { useProfileStore } from '@/stores/profileStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useLlmStore } from '@/stores/llmStore';
import { useUiStore } from '@/stores/uiStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import { useAutoLock } from '@/hooks/useAutoLock';

const settingsRequests = createSessionRequests();

// 其余 Store 与搜索缓存已各自在会话变更时清理，避免锁定事件重复维护全量名单。
onRequestSessionChange(() => {
  useLlmStore.getState().reset();
  resetPrefetchRegistry();
});

export function useSessionLifecycle({
  navigate,
  checkHasAccount,
  hasAccount,
  backendError,
  isAuthenticated,
  accountId,
}: {
  navigate: NavigateFunction;
  checkHasAccount: () => Promise<void>;
  hasAccount: boolean | null;
  backendError: boolean;
  isAuthenticated: boolean;
  accountId: string | undefined;
}) {
  const { t } = useTranslation(['settings']);

  useEffect(() => {
    setGlobalNavigate(navigate);
    return () => setGlobalNavigate(null);
  }, [navigate]);

  useEffect(() => {
    warmupPrefetchRegistry('mount');
  }, []);

  useEffect(() => {
    void checkHasAccount();
  }, [checkHasAccount]);

  useEffect(() => {
    if (hasAccount !== null) return dismissStartupScreen();
    if (backendError) window.__SOLOSOUL_STARTUP__?.fail('backend-unavailable');
  }, [hasAccount, backendError]);

  // 登录后检查 SAF 目录。无效时保留常驻横幅，并按会话去重提示。
  useEffect(() => {
    if (!isAuthenticated) return;
    const checkVaultDir = async () => {
      try {
        const { checkVaultDirectory } = await import('@/lib/vaultDirectory');
        const valid = await checkVaultDirectory();
        if (valid) {
          const ui = useUiStore.getState();
          ui.setSafAuthRevoked(false);
          ui.setSafAuthToastShown(false);
          ui.setSafSyncError(null);
          ui.setSafSyncState('idle');
          return;
        }
        logger.warn('[AppRoutes] SAF vault directory access revoked');
        const ui = useUiStore.getState();
        ui.setSafAuthRevoked(true);
        if (!ui.safAuthToastShown) {
          ui.setSafAuthToastShown(true);
          ui.showToast({
            type: 'warning',
            message: t(
              'settings:vault_directory_invalid_toast',
              'SAF directory access revoked. Go to Settings > Data Management to re-select.',
            ),
            duration: 10000,
          });
        }
        await confirmWithPause(
          t(
            'settings:vault_directory_invalid_message',
            '您之前使用的外部存储目录已被删除或无法访问。\n\nSoloSoul 已将您的数据保留在本地应用存储中，您可以继续正常使用。\n\n如需重新选择外部目录，请前往「设置 > 数据管理」。',
          ),
          {
            title: t('settings:vault_directory_invalid_title', '存储目录不可用'),
            kind: 'warning',
          },
        );
      } catch {
        // 非 Android 环境或原生对话框不可用时忽略。
      }
    };
    void checkVaultDir();
  }, [isAuthenticated, t]);

  useEffect(() => {
    if (!isAuthenticated) {
      resetPrefetchRegistry();
      return;
    }
    warmupPrefetchRegistry('afterAuth');
  }, [isAuthenticated]);

  // 设置须先于自定义页面加载，且旧会话的异步结果不得回填新账户。
  useEffect(() => {
    const account = useAuthStore.getState().currentAccount;
    let active = true;
    const request = settingsRequests.begin('settings-chain', account?.id);
    if (isAuthenticated && account) {
      void useProfileStore.getState().loadProfile(account.id);
      useSettingsStore
        .getState()
        .loadSettings(account.id)
        .then(async () => {
          if (!active || !request.isCurrent()) return;
          const s = useSettingsStore.getState().settings;
          const resolvedSystemTheme = s.theme === 'system' ? await getSystemTheme() : undefined;
          if (!active || !request.isCurrent()) return;
          await applyTheme({
            preset:
              s.theme === 'dark'
                ? 'warm-stone-dark'
                : s.theme === 'light'
                  ? 'warm-stone-light'
                  : 'system',
            accentColor: s.accentColor,
            customAccentHex: s.customAccentHex,
            backgroundType: s.backgroundType,
            backgroundValue: s.backgroundValue,
            defaultLightTheme: s.defaultLightTheme,
            defaultDarkTheme: s.defaultDarkTheme,
            resolvedSystemTheme:
              typeof resolvedSystemTheme === 'string' ? resolvedSystemTheme : undefined,
          });
          if (!active || !request.isCurrent()) return;
          try {
            await useSettingsStore.getState().loadCustomPages(account.id);
          } catch (err) {
            logger.warn('[AppRoutes] Failed to load custom pages:', err);
          }
        })
        .catch((err) => {
          if (active && request.isCurrent()) logger.warn('[AppRoutes] settings load failed:', err);
        });
    }
    return () => {
      active = false;
    };
  }, [isAuthenticated, accountId]);

  useApplyThemeFromSettings();
  useAutoLock();

  useEffect(() => {
    if (!isAuthenticated) return;
    return trackAsyncListener(
      initLlmNotificationListener().catch((err) => {
        logger.warn('[AppRoutes] LLM notification listener failed:', err);
        return null;
      }),
    );
  }, [isAuthenticated]);

  useEffect(() => {
    let active = true;
    let handling = false;
    const dispose = trackAsyncListener(
      listen('vault-locked', () => {
        if (!active || handling || !useAuthStore.getState().isAuthenticated) return;
        handling = true;
        // logout 同步清认证态；Store 自己的会话订阅只执行一次敏感状态清理。
        void useAuthStore.getState().logout();
        void useAuthStore
          .getState()
          .checkHasAccount()
          .then(() => {
            if (active && !useAuthStore.getState().isAuthenticated) navigate('/login');
          })
          .finally(() => {
            handling = false;
          });
      }),
    );
    return () => {
      active = false;
      dispose();
    };
  }, [navigate]);
}
