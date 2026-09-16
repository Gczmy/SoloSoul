import { useCallback } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { isAndroidSync } from '@/lib/platform';

/** 安卓页面退出时回到本次入口；回退历史也会恢复入口自身的 state，支持多级返回。
 * 仅用于页面退出，不替代页面内的关闭浮层、取消编辑等操作。
 */
export function useEntryBack(fallback: () => void) {
  const navigate = useNavigate();
  const { state } = useLocation();
  const from = (state as { from?: string } | null)?.from;

  return useCallback(() => {
    if (isAndroidSync() && from?.startsWith('/') && !from.startsWith('//')) {
      if (window.history.state?.idx > 0) navigate(-1);
      else navigate(from, { replace: true });
    } else {
      fallback();
    }
  }, [navigate, from, fallback]);
}
