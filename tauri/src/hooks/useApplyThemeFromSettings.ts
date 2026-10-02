import { useEffect } from 'react';
import { prepareThemeController } from '@/lib/appThemeController';

/** 仅由应用根持有；页面只保存偏好，不另开主题请求或系统监听。 */
export function useApplyThemeFromSettings() {
  useEffect(() => prepareThemeController().acquire(), []);
}
