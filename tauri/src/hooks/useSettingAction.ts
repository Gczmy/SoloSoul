import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  useSettingsStore,
  type AppSettings,
  type SettingWriteResult,
} from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';

/** 设置写入统一反馈；过期结果不打扰当前账户，也不作为成功交给调用者。 */
export function useSettingAction() {
  const updateSetting = useSettingsStore((s) => s.updateSetting);
  const showToast = useUiStore((s) => s.showToast);
  const { t } = useTranslation('common');

  return useCallback(
    async <K extends keyof AppSettings>(
      accountId: string,
      key: K,
      value: AppSettings[K],
    ): Promise<SettingWriteResult> => {
      const result = await updateSetting(accountId, key, value);
      if (result.status === 'stale' || !result.isCurrent()) return { status: 'stale' };
      if (result.status === 'failed') {
        showToast({ type: 'error', message: t('common:save_failed') });
      }
      return result;
    },
    [updateSetting, showToast, t],
  );
}
