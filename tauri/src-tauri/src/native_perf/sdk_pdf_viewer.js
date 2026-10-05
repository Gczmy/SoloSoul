// 只读候选状态。API存在或load-success本身不作为性能/最终渲染验收。
(async () => {
  const { expectedUrl, expectedPdfUrl } = __REQUEST__;
  const documentMatches = location.href === expectedUrl;
  const viewers = documentMatches ? [...document.querySelectorAll('pdf-viewer')] : [];
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
  // 只返回结构与几何；不读文本、id、class、HTML或原始URL。
  const structure = () => {
    const counts = { embed: 0, object: 0, iframe: 0, canvas: 0, pdfViewer: 0, customElements: 0 };
    const customTags = [],
      embeds = [],
      canvases = [],
      roots = [document];
    let scannedElements = 0,
      openShadowRoots = 0,
      truncated = false;
    const size = (v) => (Number.isFinite(v) ? Math.min(16384, Math.max(0, Math.round(v))) : 0);
    const visible = (el) => {
      const rect = el.getBoundingClientRect(),
        style = getComputedStyle(el);
      return (
        rect.width > 0 &&
        rect.height > 0 &&
        rect.right > 0 &&
        rect.bottom > 0 &&
        rect.left < innerWidth &&
        rect.top < innerHeight &&
        style.display !== 'none' &&
        style.visibility !== 'hidden'
      );
    };
    const sourceClass = (source) => {
      if (source === expectedPdfUrl) return 'owned-pdf';
      if (!source || source === 'about:blank') return 'blank';
      try {
        const u = new URL(source);
        if (
          u.protocol === 'chrome-extension:' &&
          /^[a-p]{32}$/.test(u.hostname) &&
          !u.username &&
          !u.password
        )
          return 'component-extension';
      } catch {
        /* 保留分类，不返回原始值。 */
      }
      return 'other';
    };
    for (let ri = 0; ri < roots.length; ri++) {
      const walker = document.createTreeWalker(roots[ri], NodeFilter.SHOW_ELEMENT);
      for (let el; (el = walker.nextNode()); ) {
        if (scannedElements === 512) {
          truncated = true;
          break;
        }
        scannedElements++;
        const tag = el.localName;
        if (tag === 'pdf-viewer') counts.pdfViewer++;
        if (tag.includes('-')) {
          counts.customElements++;
          if (/^[a-z][a-z0-9-]{0,63}$/.test(tag) && !customTags.includes(tag)) {
            if (customTags.length < 32) customTags.push(tag);
            else truncated = true;
          }
        }
        if (['embed', 'object', 'iframe'].includes(tag)) {
          counts[tag]++;
          if (embeds.length < 8) {
            const rect = el.getBoundingClientRect();
            embeds.push({
              kind: tag,
              sourceClass: sourceClass(tag === 'object' ? el.data : el.src),
              visible: visible(el),
              width: size(rect.width),
              height: size(rect.height),
            });
          } else truncated = true;
        }
        if (tag === 'canvas') {
          counts.canvas++;
          if (canvases.length < 8)
            canvases.push({ width: size(el.width), height: size(el.height), visible: visible(el) });
          else truncated = true;
        }
        if (el.shadowRoot) {
          if (openShadowRoots < 8) {
            roots.push(el.shadowRoot);
            openShadowRoots++;
          } else truncated = true;
        }
      }
      if (scannedElements === 512) {
        if (ri + 1 < roots.length) truncated = true;
        break;
      }
    }
    return {
      schemaVersion: 1,
      scope: 'windows-native-sdk-pdf-structure',
      documentReadyState: document.readyState,
      visibilityState: document.visibilityState,
      scannedElements,
      openShadowRoots,
      truncated,
      counts,
      customTags,
      embeds,
      canvases,
    };
  };
  return {
    schemaVersion: 2,
    scope: 'windows-native-sdk-pdf-viewer-candidate',
    documentMatches,
    viewerPresent: Boolean(viewer),
    loadSucceededMethodPresent,
    loadSucceeded,
    documentDimensionsPresent: pageCount !== null,
    pageCount,
    paintedFrames,
    timeOriginMs: performance.timeOrigin,
    structure: documentMatches ? structure() : null,
  };
})();
