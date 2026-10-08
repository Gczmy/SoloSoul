/** iOS 的 dvh 不扣除软键盘；根容器保持固定，正文使用实际可视高度滚动。
 * 不改变输入焦点、字段值或内部滚动位置，也不为浮层引入 transform 定位祖先。
 */
export function installIOSViewport(): () => void {
  const root = document.documentElement;
  const viewport = window.visualViewport;
  if (root.dataset.platform !== 'ios' || !viewport) return () => {};

  const update = () => {
    const height = Math.min(window.innerHeight, viewport.height);
    root.style.setProperty('--viewport-height', `${height}px`);
    // WKWebView 聚焦输入框时仍可能平移可视视口；抵消外壳偏移，内部滚动不变。
    root.style.setProperty('--ios-viewport-offset', `${viewport.offsetTop}px`);
    root.style.setProperty('--ios-keyboard-inset', `${Math.max(0, window.innerHeight - height)}px`);
  };
  update();
  viewport.addEventListener('resize', update);
  viewport.addEventListener('scroll', update);
  window.addEventListener('resize', update);
  return () => {
    viewport.removeEventListener('resize', update);
    viewport.removeEventListener('scroll', update);
    window.removeEventListener('resize', update);
    root.style.removeProperty('--viewport-height');
    root.style.removeProperty('--ios-viewport-offset');
    root.style.removeProperty('--ios-keyboard-inset');
  };
}
