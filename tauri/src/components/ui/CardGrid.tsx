import type { ReactNode } from 'react';
import styles from './CardGrid.module.css';

interface CardGridProps {
  children: ReactNode;
  className?: string;
}

/**
 * 统一卡片网格布局。
 *
 * 使用 CSS token `var(--card-grid-min-width)` 与 `var(--card-grid-gap)`，
 * 保证首页、外观设置等使用卡片网格的页面宽度/间距一致，且日后可统一调整。
 */
export function CardGrid({ children, className }: CardGridProps) {
  const classes = [styles.grid, className].filter(Boolean).join(' ');
  return <div className={classes}>{children}</div>;
}
