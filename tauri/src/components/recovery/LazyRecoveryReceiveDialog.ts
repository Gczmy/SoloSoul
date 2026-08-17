import { lazy } from 'react';

/**
 * RecoveryReceiveDialog 的共享懒加载封装（P015-R3 收敛）。
 *
 * 三处入口（OnboardingDialog / AccountSourceOverlay / LoginPage）此前各自内联
 * 同一份 lazy 工厂，统一收敛于此——若 RecoveryReceiveDialog 的命名导出被重命名，
 * 不会再有多个副本静默漂移；该映射由 LazyRecoveryReceiveDialog.test.tsx 以真实
 * 模块解析 + 渲染覆盖（重命名 → default 解析为 undefined → React 抛错 → 测试失败）。
 */
export const LazyRecoveryReceiveDialog = lazy(() =>
  import('./RecoveryReceiveDialog').then((m) => ({
    default: m.RecoveryReceiveDialog,
  })),
);
