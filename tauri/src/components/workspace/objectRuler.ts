/** 用 getElementById 定位，避免对象 ID 中的特殊字符被当成 CSS 选择器。 */
export function objectRulerAnchorId(objectId: string) {
  return `workspace-object-${objectId}`;
}

export function prefersRulerReducedMotion() {
  return (
    document.documentElement.dataset.reduceMotion === 'true' ||
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

export function shellInset(edge: 'top' | 'bottom') {
  return (
    parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue(`--shell-content-${edge}`),
    ) || 0
  );
}
