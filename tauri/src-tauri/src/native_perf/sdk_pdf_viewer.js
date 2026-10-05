// 只读候选状态。API存在或load-success本身不作为性能/最终渲染验收。
(async () => {
  const { expectedUrl } = __REQUEST__;
  const documentMatches = location.href === expectedUrl;
  const viewers = [...document.querySelectorAll('pdf-viewer')];
  const viewer = documentMatches && viewers.length === 1 ? viewers[0] : null;
  const loadSucceededMethodPresent = typeof viewer?.getLoadSucceededForTesting === 'function';
  let loadSucceeded = null;
  if (loadSucceededMethodPresent) {
    try {
      const result = viewer.getLoadSucceededForTesting();
      if (typeof result === 'boolean') loadSucceeded = result;
    } catch {
      /* 只保留不可用，不返回异常内容。 */
    }
  }
  const dims = viewer?.documentDimensions;
  const pageCount =
    Array.isArray(dims?.pageDimensions) && dims.pageDimensions.length <= 100
      ? dims.pageDimensions.length
      : null;
  let paintedFrames = 0,
    timer;
  if (loadSucceeded === true)
    await Promise.race([
      new Promise((resolve) =>
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            paintedFrames = 2;
            resolve();
          }),
        ),
      ),
      new Promise((resolve) => {
        timer = setTimeout(resolve, 1000);
      }),
    ]);
  clearTimeout(timer);
  return {
    schemaVersion: 1,
    scope: 'windows-native-sdk-pdf-viewer-candidate',
    documentMatches,
    viewerPresent: Boolean(viewer),
    loadSucceededMethodPresent,
    loadSucceeded,
    documentDimensionsPresent: pageCount !== null,
    pageCount,
    paintedFrames,
    timeOriginMs: performance.timeOrigin,
  };
})();
