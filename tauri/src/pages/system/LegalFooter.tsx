/**
 * 版权页脚（P224-④ 拆分）。静态内容，无依赖。
 */
export function LegalFooter() {
  return (
    <div
      style={{
        textAlign: 'center',
        padding: '8px 0',
        fontSize: 'var(--text-badge)',
        color: 'var(--text-tertiary)',
        lineHeight: 1.8,
      }}
    >
      <div>Copyright &copy; {new Date().getFullYear()} SoloSoul</div>
      <div>MIT License &mdash; Open Source Software</div>
    </div>
  );
}
