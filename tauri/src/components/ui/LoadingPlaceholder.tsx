import { memo } from 'react';
import styles from './LoadingPlaceholder.module.css';

interface LoadingPlaceholderProps {
  /** 覆盖背景色，默认使用页面级背景 */
  variant?: 'base' | 'elevated' | 'toolbar';
  /** 最小高度，用于卡片/列表占位 */
  minHeight?: number | string;
  className?: string;
  style?: React.CSSProperties;
}

/**
 * 纯色加载占位，不显示任何文字、图标或动画。
 * 用于页面/数据初次加载，最大限度降低画面闪烁。
 */
export const LoadingPlaceholder = memo(function LoadingPlaceholder({
  variant = 'base',
  minHeight,
  className = '',
  style,
}: LoadingPlaceholderProps) {
  return (
    <div
      data-testid="loading-placeholder"
      className={`${styles.placeholder} ${styles[variant]} ${className}`}
      style={{ ...style, minHeight }}
      aria-hidden="true"
    />
  );
});
