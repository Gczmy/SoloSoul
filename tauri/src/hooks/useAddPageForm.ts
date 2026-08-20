/**
 * AddPageButton 的「新建页面」表单域：名称/描述/图标状态、重名校验、创建提交。
 */
import { useState, useCallback } from 'react';
import type { TFunction } from 'i18next';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { CustomPage } from '@/stores/settingsStore';
import { DEFAULT_CUSTOM_ICON, type CustomIconId } from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS } from '@/components/layout/useNavigationItems';

export interface UseAddPageFormOptions {
  /** 创建成功回调（父组件负责导航等）。 */
  onCreate: (page: CustomPage) => void;
  t: TFunction;
  onError: (err: unknown, context: string) => void;
}

export function useAddPageForm({ onCreate, t, onError }: UseAddPageFormOptions) {
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const addCustomPage = useSettingsStore((s) => s.addCustomPage);

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [nameError, setNameError] = useState<'empty' | 'duplicate' | null>(null);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(DEFAULT_CUSTOM_ICON);

  /** 重置表单字段（不关闭弹层——由父组件负责）。 */
  const handleCancel = useCallback(() => {
    setName('');
    setDescription('');
    setNameError(null);
    setSelectedIconId(DEFAULT_CUSTOM_ICON);
  }, []);

  /**
   * 确认创建。返回是否应关闭弹层：错误路径（显式空名称/重名）返回 false 留在弹层
   * 展示错误；成功或隐式空名称取消返回 true（父组件据此收起弹层）。
   */
  const handleConfirm = useCallback(
    (isExplicit = false): boolean => {
      const trimmed = name.trim();
      if (!trimmed || !currentAccount) {
        if (isExplicit) {
          setNameError('empty');
          return false;
        }
        handleCancel();
        return true;
      }
      // Check for duplicate page names
      const store = useSettingsStore.getState();
      const existingNames = [
        ...SYSTEM_PAGE_KEYS.map((k) => t(k)),
        ...store.settings.customPages.filter((p) => !p.deletedAt).map((p) => p.name),
      ];
      if (existingNames.some((n) => n.toLowerCase() === trimmed.toLowerCase())) {
        setNameError('duplicate');
        return false;
      }
      const trimmedDesc = description.trim();
      addCustomPage(currentAccount.id, trimmed, selectedIconId, trimmedDesc || undefined)
        .then((page) => {
          onCreate(page);
        })
        // P003: 创建失败（store 已回滚并抛错）——提示错误，不导航到不存在的页面。
        .catch((err) => {
          onError(err, t('navigation:add_page_failed', { defaultValue: '创建页面失败' }));
        });
      handleCancel();
      return true;
    },
    [name, description, selectedIconId, currentAccount, addCustomPage, onCreate, onError, t, handleCancel],
  );

  return {
    name,
    description,
    nameError,
    selectedIconId,
    setName,
    setDescription,
    setNameError,
    setSelectedIconId,
    handleCancel,
    handleConfirm,
  };
}
