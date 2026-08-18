import styles from './ContentLoadingSkeleton.module.css';

/**
 * 内容区骨架：页面 chunk 拉取期在常驻壳的内容区显示（替代旧 RouteLoadingSkeleton
 * 的整体假壳——壳已常驻，这里只占内容区，结构对齐真实页面内容）。
 * 纯装饰，aria-hidden，无任何交互。
 */
export function ContentLoadingSkeleton() {
  return (
    <div className={styles.content} data-testid="content-loading-skeleton" aria-hidden="true">
      <div className={styles.bar} style={{ width: '42%' }} />
      <div className={styles.bar} style={{ width: '78%' }} />
      <div className={styles.bar} style={{ width: '56%' }} />
      <div className={styles.bar} style={{ width: '92%' }} />
      <div className={styles.card} />
      <div className={styles.card} />
    </div>
  );
}
