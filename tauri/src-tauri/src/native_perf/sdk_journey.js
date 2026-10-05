// RF-312 SDK固定UI探针：只返回公开fixture的结构、坐标与无payload的IPC计数。
// 输入由原生Input协议发送；这里不调用click、写value或调用业务IPC。
(async () => {
  const { step, runId, objectCount, readOnlyStartup = false } = __REQUEST__;
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
  const targets = {
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
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            if (Object.hasOwn(conditions, step) ? conditions[step]() : visible(targets[step]()))
              resolve('ready');
            else check();
          }),
        );
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
