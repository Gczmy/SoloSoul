// 固定公开PDF的主文档结构与可信输入检查；不触发业务读取或DOM输入。
(async () => {
  const { phase, runId, timeOriginMs, pdfUrl } = __REQUEST__;
  if (!['target', 'opened', 'closed'].includes(phase)) throw new Error('unsupported-pdf-phase');
  const mainVerified = () =>
    window.top === window &&
    window.__SOLOSOUL_NATIVE_PERF_RUN_ID__ === runId &&
    performance.timeOrigin === timeOriginMs &&
    location.href === 'http://tauri.localhost/settings/attachments';
  const visible = (node) => {
    if (!node) return false;
    const r = node.getBoundingClientRect(),
      s = getComputedStyle(node);
    return r.width > 0 && r.height > 0 && s.visibility === 'visible' && s.display !== 'none';
  };
  const unique = (items) => (items.length === 1 ? items[0] : null);
  const overlay = () =>
    unique(
      [...document.querySelectorAll('[data-testid="attachment-preview-overlay"]')].filter(visible),
    );
  const pdf = () =>
    overlay() &&
    unique([...overlay().querySelectorAll('embed[title="text_only.pdf"]')].filter(visible));
  const target = () => {
    const box = unique(
      [...document.querySelectorAll('input[type="checkbox"]')].filter(
        (n) => n.getAttribute('aria-label') === 'text_only.pdf',
      ),
    );
    let row = box?.parentElement;
    while (row && row !== document.body) {
      const buttons = [...row.querySelectorAll('button[title="Preview"]')].filter(visible);
      if (buttons.length === 1) return buttons[0];
      if (buttons.length > 1) return null;
      row = row.parentElement;
    }
    return null;
  };
  const close = () =>
    overlay() &&
    unique(
      [...overlay().querySelectorAll('button')].filter(
        (n) =>
          visible(n) &&
          (n.getAttribute('aria-label') === 'Close' || n.textContent.trim() === 'Close'),
      ),
    );
  const point = (node) => {
    if (!visible(node)) return null;
    const r = node.getBoundingClientRect(),
      x = r.x + r.width / 2,
      y = r.y + r.height / 2;
    const hit = document.elementFromPoint(x, y);
    return {
      x,
      y,
      actionable:
        x >= 0 &&
        y >= 0 &&
        x < innerWidth &&
        y < innerHeight &&
        !node.disabled &&
        (hit === node || node.contains(hit)),
    };
  };
  const exactPdf = () =>
    pdf()?.getAttribute('src') === pdfUrl &&
    pdf()?.getAttribute('type') === 'application/pdf' &&
    document.querySelectorAll('embed').length === 1;
  const condition = () =>
    mainVerified() &&
    (phase === 'target'
      ? !overlay() && visible(target()) && document.querySelectorAll('iframe,frame').length === 0
      : phase === 'opened'
        ? exactPdf() && visible(close())
        : !overlay() && !document.querySelector('embed,iframe,frame'));
  let timer,
    finished = false;
  const outcome = await Promise.race([
    new Promise((resolve) => {
      const check = () => {
        if (finished) return;
        if (!mainVerified()) {
          resolve('document-mismatch');
          return;
        }
        if (condition())
          requestAnimationFrame(() =>
            requestAnimationFrame(() => (condition() ? resolve('ready') : check())),
          );
        else requestAnimationFrame(check);
      };
      check();
    }),
    new Promise((resolve) => {
      timer = setTimeout(() => resolve('timeout'), 8000);
    }),
  ]);
  finished = true;
  clearTimeout(timer);
  return {
    schemaVersion: 1,
    scope: 'windows-native-sdk-pdf-dom',
    phase,
    outcome,
    mainVerified: mainVerified(),
    embedded: phase === 'opened' && exactPdf(),
    frameElementCount: document.querySelectorAll('iframe,frame').length,
    atMs: performance.now(),
    target:
      outcome === 'ready' && phase !== 'closed'
        ? point(phase === 'target' ? target() : close())
        : null,
    inputTrust: window.__SOLOSOUL_SDK_JOURNEY_INPUTS__?.snapshot(),
  };
})();
