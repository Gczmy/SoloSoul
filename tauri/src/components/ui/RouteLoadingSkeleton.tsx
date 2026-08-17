import styles from './RouteLoadingSkeleton.module.css';

/**
 * 路由切换骨架屏：页面 chunk 拉取期间替代纯色占位（LoadingPlaceholder）。
 * 保持侧边栏（桌面）/底部导航（移动）与内容区的视觉结构，配合轻微 shimmer，
 * 避免「整窗空白」的卡顿感知；纯装饰，aria-hidden，无任何交互。
 */
export function RouteLoadingSkeleton() {
  return (
    <div className={styles.shell} data-testid="route-loading-skeleton" aria-hidden="true">
      <aside className={styles.side}>
        <div className={styles.sideLogo} />
        {[0, 1, 2, 3, 4, 5].map((i) => (
          <div key={i} className={styles.sideIcon} />
        ))}
      </aside>
      <main className={styles.main}>
        <div className={styles.top}>
          <div className={`${styles.bar} ${styles.title}`} />
        </div>
        <div className={styles.content}>
          <div className={styles.bar} style={{ width: '42%' }} />
          <div className={styles.bar} style={{ width: '78%' }} />
          <div className={styles.bar} style={{ width: '56%' }} />
          <div className={styles.bar} style={{ width: '92%' }} />
          <div className={styles.card} />
          <div className={styles.card} />
        </div>
        <nav className={styles.bottom}>
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className={styles.bottomIcon} />
          ))}
        </nav>
      </main>
    </div>
  );
}
