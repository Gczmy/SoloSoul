/**
 * AddPageButton 的「新建页面」表单域：名称/描述/图标状态、重名校验、创建提交。
 */
import { useState, useCallback, useEffect, useRef } from 'react';
import type { TFunction } from 'i18next';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { CustomPage } from '@/stores/settingsStore';
import { DEFAULT_CUSTOM_ICON, type CustomIconId } from '@/lib/pageIcons';
import { onRequestSessionChange } from '@/lib/sessionRequests';
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
  const activeSubmit = useRef<object | null>(null);

  /** 重置表单字段（不关闭弹层——由父组件负责）。 */
  const handleCancel = useCallback(() => {
    activeSubmit.current = null;
    setName('');
    setDescription('');
    setNameError(null);
    setSelectedIconId(DEFAULT_CUSTOM_ICON);
  }, []);

  useEffect(() => {
    handleCancel();
    return onRequestSessionChange(handleCancel);
  }, [currentAccount?.id, handleCancel]);

  /**
   * 确认创建。返回是否已开始提交或隐式取消；校验失败与重复提交返回 false。
   * 提交结果由回调处理，父组件只能在创建成功后收起弹层。
   */
  const handleConfirm = useCallback(
    (isExplicit = false): boolean => {
      if (activeSubmit.current) return false;
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
      const request = {};
      activeSubmit.current = request;
      const isCurrent = () =>
        activeSubmit.current === request &&
        useAuthStore.getState().isAuthenticated &&
        useAuthStore.getState().currentAccount?.id === currentAccount.id;
      void addCustomPage(currentAccount.id, trimmed, selectedIconId, trimmedDesc || undefined)
        .then((page) => {
          if (!isCurrent()) return;
          handleCancel();
          onCreate(page);
        })
        // P003: 创建失败（store 已回滚并抛错）——提示错误，不导航到不存在的页面。
        .catch((err) => {
          if (isCurrent())
            onError(err, t('navigation:add_page_failed', { defaultValue: '创建页面失败' }));
        })
        .finally(() => {
          if (activeSubmit.current === request) activeSubmit.current = null;
        });
      return true;
    },
    [
      name,
      description,
      selectedIconId,
      currentAccount,
      addCustomPage,
      onCreate,
      onError,
      t,
      handleCancel,
    ],
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
