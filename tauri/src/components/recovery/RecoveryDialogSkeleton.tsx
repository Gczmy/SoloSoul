import styles from './RecoveryDialogSkeleton.module.css';

/**
 * 恢复对话框的轻量骨架占位（P015-R3-R3）。
 *
 * 冷开（恢复对话框 chunk + html5-qrcode ~400K 首次按需加载）期间替代
 * Suspense fallback={null}，避免入口卡片隐藏后的短暂空白。
 * 纯 CSS shimmer，零第三方依赖——framer-motion 本身也在懒加载链上，
 * 骨架不得引入任何可能延迟自身的库。
 *
 * 无障碍：与 RouteLoadingSkeleton 同一策略——整体 aria-hidden（纯装饰占位，
 * 真实对话框出现后自然接管读屏焦点），不引入硬编码文案（规避 i18n 纪律问题）。
 */
export function RecoveryDialogSkeleton() {
  return (
    <div className={styles.overlay} aria-hidden="true">
      <div className={styles.card}>
        <div className={styles.title} />
        <div className={styles.tabs}>
          <div className={styles.tab} />
          <div className={styles.tab} />
        </div>
        <div className={`${styles.line} ${styles.lineWide}`} />
        <div className={`${styles.line} ${styles.lineNarrow}`} />
        <div className={styles.box} />
      </div>
    </div>
  );
}
