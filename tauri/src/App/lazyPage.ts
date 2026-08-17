import { lazy } from 'react';
import type { ComponentType, LazyExoticComponent } from 'react';

/**
 * 将「具名导出的页面模块」收敛为 React.lazy 组件（P015-R4）。
 *
 * 消除 `lazy(() => loadXxx().then((m) => ({ default: m.Xxx })))` 的 28 处样板，
 * 并与 routeLoaders 预取共用同一 loader（懒加载 / 预取单真相源）。
 * `exportName` 以 `keyof M` 约束——命名导出被重命名时此处即编译报错，
 * 无需额外的运行时漂移保护测试（对比 LazyPhotoViewerOverlay /
 * LazyRecoveryReceiveDialog 的运行时映射测试）。
 */
export function lazyPage<M extends object>(
  loader: () => Promise<M>,
  exportName: keyof M & string,
): LazyExoticComponent<ComponentType<unknown>> {
  return lazy(() =>
    loader().then((m) => ({
      default: m[exportName] as unknown as ComponentType<unknown>,
    })),
  );
}
