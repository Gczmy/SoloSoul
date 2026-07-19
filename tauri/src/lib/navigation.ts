import type { NavigateFunction } from 'react-router-dom';

let globalNavigate: NavigateFunction | null = null;

/**
 * 注册全局 navigate 函数，供 React 组件树外部的模块使用。
 * 应在 AppRoutes 等 Router 内部组件初始化时调用。
 */
export function setGlobalNavigate(fn: NavigateFunction | null): void {
  globalNavigate = fn;
}

/**
 * 使用 react-router 的 navigate 进行客户端跳转。
 * 若尚未注册，则回退到 window.location.href。
 */
export function navigateTo(path: string): void {
  if (globalNavigate) {
    globalNavigate(path);
  } else {
    window.location.href = path;
  }
}
