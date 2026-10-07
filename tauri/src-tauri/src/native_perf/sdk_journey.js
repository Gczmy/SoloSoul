// RF-312 SDK固定UI探针：只返回公开fixture的结构、坐标与无payload的IPC计数。
// 输入由原生Input协议发送；这里不调用click、写value或调用业务IPC。
(async () => {
  const {
    step,
    runId,
    objectCount,
    readOnlyStartup = false,
    mediaJourney = false,
    ocrJourney = false,
  } = __REQUEST__;
  const expectedCards = Math.min(50, Math.ceil(objectCount / 20));
  const visible = (node) => {
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = getComputedStyle(node);
    return (
      rect.width > 0 &&
      rect.height > 0 &&
      style.visibility === 'visible' &&
      style.display !== 'none'
    );
  };
  const unique = (nodes) => (nodes.length === 1 ? nodes[0] : null);
  const button = (parent, label) =>
    unique(
      [...document.querySelectorAll(parent + ' button')].filter(
        (node) =>
          visible(node) &&
          (node.getAttribute('aria-label') === label || node.textContent.trim() === label),
      ),
    );
  const input = () =>
    unique(
      [...document.querySelectorAll('[data-login-method-region="password"] input')].filter(visible),
    );
  const home = () =>
    location.pathname === '/' && visible(document.getElementById('desktop-navigation'));
  const locked = () =>
    location.pathname === '/login' &&
    visible(input()) &&
    !document.getElementById('desktop-navigation') &&
    document.querySelectorAll('[data-login-card]').length === 1 &&
    document.querySelector('[data-login-card]').textContent.includes('Performance Fixture');
  const publicText = 'RF-312 public synthetic attachment preview.\nHello SoloSoul 1234567890.\n';
  const overlay = () =>
    unique(
      [...document.querySelectorAll('[data-testid="attachment-preview-overlay"]')].filter(visible),
    );
  const previewImage = () =>
    overlay() && unique([...overlay().querySelectorAll('img[alt="ocr_test.png"]')].filter(visible));
  const previewText = () =>
    overlay() && unique([...overlay().querySelectorAll('pre')].filter(visible));
  const mediaCard = () =>
    unique(
      [...document.querySelectorAll('[data-shell-content] [data-ui-card][role="button"]')].filter(
        (node) =>
          visible(node) &&
          [...node.querySelectorAll('h1,h2,h3,h4,h5,h6')].some(
            (title) => title.textContent.trim() === 'Attachments',
          ),
      ),
    );
  const previewButton = (name) => {
    const checkbox = unique(
      [...document.querySelectorAll('input[type="checkbox"]')].filter(
        (node) => node.getAttribute('aria-label') === name,
      ),
    );
    let row = checkbox?.parentElement;
    while (row && row !== document.body) {
      const buttons = [...row.querySelectorAll('button[title="Preview"]')].filter(visible);
      if (buttons.length === 1) return buttons[0];
      if (buttons.length > 1) return null;
      row = row.parentElement;
    }
    return null;
  };
  const ocrCard = () =>
    unique(
      [...document.querySelectorAll('[data-shell-content] [data-ui-card][role="button"]')].filter(
        (node) =>
          visible(node) &&
          [...node.querySelectorAll('h1,h2,h3,h4,h5,h6')].some(
            (title) => title.textContent.trim() === 'OCR',
          ),
      ),
    );
  const ocrResultText = () => {
    if (location.pathname !== '/ocr') return null;
    const cards = [...document.querySelectorAll('[data-shell-content] [data-ui-card]')].filter(
      (card) => [...card.querySelectorAll('h3')].some((h) => h.textContent.trim() === 'Result'),
    );
    const card = unique(cards);
    if (!card) return null;
    return unique(
      [...card.querySelectorAll('div')].filter(
        (node) =>
          node.style.whiteSpace === 'pre-wrap' &&
          visible(node) &&
          node.textContent.length <= 256 &&
          node.textContent.replace(/[^a-z0-9]/gi, '').toLowerCase() ===
            'helloppocrv6solosoulocr1234567890',
      ),
    );
  };
  const resultOnscreen = () => {
    const node = ocrResultText();
    if (!node) return false;
    const r = node.getBoundingClientRect(),
      x = r.x + r.width / 2,
      y = r.y + r.height / 2,
      hit = document.elementFromPoint(x, y);
    return (
      x >= 0 && y >= 0 && x < innerWidth && y < innerHeight && (hit === node || node.contains(hit))
    );
  };
  const scans = () =>
    window.__SOLOSOUL_NATIVE_PERF__
      ?.snapshot()
      ?.commands?.filter((v) => v.command === 'ocr_scan_image').length;
  const targets = {
    attachmentsCard: mediaCard,
    ocrCard,
    ocrSelect: () => button('[data-shell-content]', 'Select Image or PDF'),
    ocrResultText,
    imagePreview: () => previewButton('ocr_test.png'),
    textPreview: () => previewButton('preview.txt'),
    previewClose: () => button('[data-testid="attachment-preview-overlay"]', 'Close'),
    password: input,
    submit: () =>
      unique([...document.querySelectorAll('[data-login-password-submit]')].filter(visible)),
    identity: () => button('#desktop-navigation', 'Identity'),
    clear: () => button('[data-shell-content]', 'Clear'),
    homeButton: () => button('#desktop-navigation', 'Home'),
    searchCard: () =>
      unique(
        [...document.querySelectorAll('[data-shell-content] [data-ui-card][role="button"]')].filter(
          (node) =>
            visible(node) &&
            [...node.querySelectorAll('h1,h2,h3,h4,h5,h6')].some(
              (title) => title.textContent.trim() === 'Search',
            ),
        ),
      ),
    searchInput: () =>
      unique(
        [...document.querySelectorAll('input[placeholder="Search objects, profiles..."]')].filter(
          visible,
        ),
      ),
    lockButton: () => button('#desktop-navigation', 'Lock Vault'),
  };
  const conditions = {
    startup: () =>
      locked() &&
      (!readOnlyStartup || (visible(targets.submit()) && !targets.submit().disabled)) &&
      performance.getEntriesByName('solosoul:startup-dismissed', 'mark').length === 1,
    home,
    ocrReady: () =>
      location.pathname === '/ocr' &&
      visible(targets.ocrSelect()) &&
      !targets.ocrSelect().disabled &&
      button('[data-shell-content]', 'General')?.style.fontWeight === '600' &&
      scans() === 0 &&
      !ocrResultText(),
    ocrResultReady: () => location.pathname === '/ocr' && resultOnscreen() && scans() === 1,
    attachments: () =>
      location.pathname === '/settings/attachments' &&
      !overlay() &&
      [...document.querySelectorAll('button[title="Preview"]')].filter(visible).length === 4 &&
      visible(targets.imagePreview()) &&
      visible(targets.textPreview()),
    imagePreviewReady: () => {
      const img = previewImage();
      return (
        location.pathname === '/settings/attachments' &&
        visible(overlay()) &&
        img?.complete === true &&
        img.naturalWidth === 500 &&
        img.naturalHeight === 200
      );
    },
    textPreviewReady: () =>
      location.pathname === '/settings/attachments' &&
      visible(overlay()) &&
      previewText()?.textContent === publicText,
    workspace: () =>
      location.pathname === '/workspace' &&
      !location.search &&
      document.querySelectorAll('[data-testid="workspace-object-card"]').length === 50,
    search: () =>
      visible(targets.searchInput()) &&
      document.querySelectorAll('[data-shell-content] [data-ui-card][role="button"]').length ===
        expectedCards,
    locked,
  };
  if (!Object.hasOwn(targets, step) && !Object.hasOwn(conditions, step))
    throw new Error('unsupported-step');
  if (
    !mediaJourney &&
    [
      'attachmentsCard',
      'imagePreview',
      'textPreview',
      'previewClose',
      'attachments',
      'imagePreviewReady',
      'textPreviewReady',
    ].includes(step)
  )
    throw new Error('unsupported-media-step');
  if (
    !ocrJourney &&
    ['ocrCard', 'ocrSelect', 'ocrResultText', 'ocrReady', 'ocrResultReady'].includes(step)
  )
    throw new Error('unsupported-ocr-step');
  const result = await new Promise((resolve) => {
    const began = performance.now();
    const check = () => {
      if (performance.now() - began >= 45000) {
        resolve('timeout');
        return;
      }
      if (
        window.__SOLOSOUL_NATIVE_PERF_RUN_ID__ !== runId ||
        window.top !== window ||
        location.origin !== 'http://tauri.localhost' ||
        document.querySelectorAll('iframe,frame').length
      ) {
        resolve('document-mismatch');
        return;
      }
      if (Object.hasOwn(conditions, step) ? conditions[step]() : visible(targets[step]())) {
        const decode =
          step === 'imagePreviewReady'
            ? previewImage()
                .decode()
                .then(
                  () => true,
                  () => false,
                )
            : Promise.resolve(true);
        decode.then((decoded) => {
          if (!decoded) {
            requestAnimationFrame(check);
            return;
          }
          requestAnimationFrame(() =>
            requestAnimationFrame(() => {
              if (Object.hasOwn(conditions, step) ? conditions[step]() : visible(targets[step]()))
                resolve('ready');
              else check();
            }),
          );
        });
      } else requestAnimationFrame(check);
    };
    check();
  });
  if (!window.__SOLOSOUL_SDK_JOURNEY_INPUTS__) {
    const counts = { pointer: 0, text: 0, untrusted: 0 };
    for (const type of ['pointerdown', 'input'])
      window.addEventListener(
        type,
        (event) => {
          if (!event.isTrusted) counts.untrusted += 1;
          else if (type === 'pointerdown') counts.pointer += 1;
          else counts.text += 1;
        },
        true,
      );
    Object.defineProperty(window, '__SOLOSOUL_SDK_JOURNEY_INPUTS__', {
      value: Object.freeze({ snapshot: () => ({ ...counts }) }),
    });
  }
  let target = null;
  if (result === 'ready' && Object.hasOwn(targets, step)) {
    const node = targets[step]();
    const rect = node.getBoundingClientRect();
    const x = rect.x + rect.width / 2,
      y = rect.y + rect.height / 2;
    const hit = document.elementFromPoint(x, y);
    target = {
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
    // 仅固定媒体行程目标可请求滚轮；这里只提供经命中检查的几何，不写 scrollTop。
    if (
      mediaJourney &&
      [
        'attachmentsCard',
        'imagePreview',
        'textPreview',
        'searchCard',
        'ocrCard',
        'ocrResultText',
      ].includes(step) &&
      !target.actionable &&
      !node.disabled
    ) {
      const content = document.querySelector('[data-shell-content]');
      if (content && content.contains(node) && content.scrollHeight > content.clientHeight) {
        const box = content.getBoundingClientRect();
        const clipTop = Math.max(0, box.top),
          clipBottom = Math.min(innerHeight, box.bottom);
        const wheelX = Math.min(innerWidth - 1, Math.max(0, box.left + box.width / 2));
        const wheelY = (clipTop + clipBottom) / 2;
        const wheelHit = document.elementFromPoint(wheelX, wheelY);
        if (
          clipBottom > clipTop &&
          (y < clipTop || y >= clipBottom) &&
          wheelHit &&
          content.contains(wheelHit)
        )
          target.scroll = {
            x: wheelX,
            y: wheelY,
            deltaY: Math.max(-600, Math.min(600, y - wheelY)),
            viewportWidth: innerWidth,
            viewportHeight: innerHeight,
            clipTop,
            clipBottom,
          };
      }
    }
  }
  const probe = {
    schemaVersion: 1,
    scope: 'windows-native-sdk-ui-probe',
    runId,
    step,
    outcome: result,
    href: location.href,
    origin: location.origin,
    timeOriginMs: performance.timeOrigin,
    atMs: performance.now(),
    rootPresent: Boolean(document.getElementById('root')?.hasChildNodes()),
    frameCount: document.querySelectorAll('iframe,frame').length,
    target,
    inputTrust: window.__SOLOSOUL_SDK_JOURNEY_INPUTS__.snapshot(),
    observer: window.__SOLOSOUL_NATIVE_PERF__?.snapshot(),
  };
  if (result === 'ready' && ['imagePreviewReady', 'textPreviewReady'].includes(step)) {
    const image = step === 'imagePreviewReady';
    probe.media = {
      kind: image ? 'image' : 'text',
      decoded: image,
      textMatches: !image,
      width: image ? 500 : 0,
      height: image ? 200 : 0,
      visible: true,
      paintedFrames: 2,
    };
  }
  if (result === 'ready' && step === 'ocrResultReady') {
    probe.ocr = {
      kind: 'fixed-public-image',
      text: ocrResultText().textContent,
      normalizedText: 'helloppocrv6solosoulocr1234567890',
      visible: true,
      paintedFrames: 2,
      tier: 'small',
      firstScanInvokeCount: scans(),
    };
  }
  if (result === 'timeout') {
    const submit = targets.submit();
    probe.timeoutState = {
      focused: document.hasFocus(),
      visibility: document.visibilityState,
      viewportWidth: innerWidth,
      viewportHeight: innerHeight,
      homeVisible: home(),
      passwordVisible: visible(input()),
      submitVisible: visible(submit),
      submitDisabled: Boolean(submit?.disabled),
    };
  }
  return probe;
})();
