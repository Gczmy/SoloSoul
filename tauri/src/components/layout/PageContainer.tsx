import type { ReactNode } from 'react';
import styles from './PageContainer.module.css';

export type PageContainerVariant = 'wide' | 'medium' | 'small' | 'xs' | 'form' | 'full';
export type PageContainerGap = 'none' | 'default' | 'section' | 'large';

interface PageContainerProps {
  variant?: PageContainerVariant;
  gap?: PageContainerGap;
  className?: string;
  children: ReactNode;
}

/**
 * 统一页面内容容器。
 *
 * 所有页面应通过本组件控制内容区最大宽度、水平内边距与垂直间距，
 * 避免每个页面重复写死 `maxWidth` / `margin: 0 auto`。
 *
 * 宽度标准（与 tokens.css 保持一致）：
 * - wide  : 720px — 首页 / 内容页 / 列表页
 * - medium: 640px — 中等宽度页
 * - small : 600px — 设置首页 / 回收站 / 附件 / 搜索
 * - xs    : 560px — 编辑器 / 历史 / 同步 / LLM 配置
 * - form  : 480px — 表单 / 设置详情页
 * - full  : 无最大宽度 — 特殊布局（如 LLM 聊天双栏）
 */
export function PageContainer({
  variant = 'wide',
  gap = 'default',
  className,
  children,
}: PageContainerProps) {
  const classes = [
    styles.container,
    styles[variant],
    gap !== 'none' && styles[`gap-${gap}`],
    className,
  ]
    .filter(Boolean)
    .join(' ');

  return <div className={classes}>{children}</div>;
}
