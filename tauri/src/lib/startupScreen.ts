/** 仅操作静态启动层，不引入 React、Store 或 IPC，避免启动依赖闭环。 */
export function dismissStartupScreen(): () => void {
  const screen = document.getElementById('startup-screen');
  if (!screen) return () => {};
  let secondFrame = 0;
  let removeTimer: ReturnType<typeof setTimeout> | undefined;
  // 确保目标页面已有一次绘制机会，再撤下覆盖层；不增加最短展示时长。
  const firstFrame = requestAnimationFrame(() => {
    secondFrame = requestAnimationFrame(() => {
      screen.dataset.ready = 'true';
      screen.setAttribute('aria-hidden', 'true');
      performance.mark('solosoul:startup-dismissed');
      const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
      removeTimer = setTimeout(() => screen.remove(), reducedMotion ? 0 : 180);
    });
  });
  return () => {
    cancelAnimationFrame(firstFrame);
    cancelAnimationFrame(secondFrame);
    clearTimeout(removeTimer);
    screen.removeAttribute('data-ready');
    screen.removeAttribute('aria-hidden');
  };
}
