import { useCallback } from 'react';
import { isAndroidSync } from '@/lib/platform';
import { useToastOutletStore } from '@/stores/toastOutletStore';
import styles from './ToastContainer.module.css';

/** Android 通知放入当前页面或浮层的正常布局流，避免遮挡底部操作。 */
export function ToastOutlet({
  priority = 0,
  className,
}: {
  priority?: number;
  className?: string;
}) {
  const register = useToastOutletStore((state) => state.register);
  const attach = useCallback(
    (node: HTMLDivElement | null) => {
      if (!node) return;
      // 浮层有自定义层级时读取真实 CSS，默认值供尚未解析样式的环境使用。
      let layer = node.parentElement;
      let resolved = priority;
      if (priority > 0) {
        while (layer) {
          const z = Number.parseFloat(getComputedStyle(layer).zIndex);
          if (Number.isFinite(z)) resolved = Math.max(resolved, z);
          layer = layer.parentElement;
        }
      }
      return register(node, resolved);
    },
    [priority, register],
  );

  if (!isAndroidSync()) return null;
  return <div ref={attach} className={`${styles.outlet} ${className ?? ''}`} data-toast-outlet />;
}
