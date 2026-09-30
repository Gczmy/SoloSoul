// RF-312 Windows 默认 Tauri transport 的条件完整计数；不读取 IPC payload。
(() => {
  'use strict';
  const key = '__SOLOSOUL_NATIVE_PERF__';
  if (window[key]) {
    window[key].invalidate('duplicate-installation');
    return;
  }
  const runId = window.__SOLOSOUL_NATIVE_PERF_RUN_ID__;
  const originalFetch = window.fetch;
  const originalWarn = console.warn;
  const maxEvents = 16384;
  const commands = [];
  const invalidReasons = new Set();
  const installedAtMs = performance.now();
  const timeOriginMs = performance.timeOrigin;
  let observedCount = 0;
  if (typeof runId !== 'string' || !/^[a-f0-9]{32}$/.test(runId)) {
    invalidReasons.add('missing-or-invalid-run-id');
  }
  function invalidate(reason) {
    invalidReasons.add(reason);
  }
  function observedFetch(...args) {
    try {
      const input = args[0];
      const options = args[1];
      const urlText =
        typeof input === 'string' || input instanceof URL ? String(input) : input?.url;
      const method = String(options?.method ?? input?.method ?? 'GET').toUpperCase();
      const url = new URL(urlText, window.location.href);
      if (url.hostname === 'ipc.localhost' && method === 'POST') {
        if (url.protocol !== 'http:' || url.port || url.username || url.password) {
          invalidate('unsupported-ipc-origin');
        }
        const command = decodeURIComponent(url.pathname.slice(1));
        if (!command) invalidate('missing-command');
        observedCount += 1;
        if (commands.length < maxEvents) {
          commands.push({ command, atMs: performance.now() });
        } else {
          invalidate('overflow');
        }
      }
    } catch {
      // 无法分类的调用不能被无声丢弃后仍宣称总量准确。
      invalidate('fetch-observation-error');
    }
    return Reflect.apply(originalFetch, this, args);
  }
  function observedWarn(...args) {
    if (
      typeof args[0] === 'string' &&
      args[0].startsWith(
        'IPC custom protocol failed, Tauri will now use the postMessage interface instead',
      )
    ) {
      invalidate('transport-fallback');
    }
    return Reflect.apply(originalWarn, this, args);
  }
  if (typeof originalFetch !== 'function' || typeof originalWarn !== 'function') {
    invalidate('missing-transport-function');
  } else {
    window.fetch = observedFetch;
    console.warn = observedWarn;
  }
  function snapshot() {
    if (performance.timeOrigin !== timeOriginMs) invalidate('document-time-origin-changed');
    if (window.top !== window) invalidate('non-main-frame');
    if (typeof document?.querySelectorAll !== 'function') invalidate('missing-frame-observation');
    else if (document.querySelectorAll('iframe,frame').length !== 0)
      invalidate('additional-frames');
    if (window.fetch !== observedFetch) invalidate('fetch-wrapper-replaced');
    if (console.warn !== observedWarn) invalidate('warn-wrapper-replaced');
    const valid = invalidReasons.size === 0;
    return {
      schemaVersion: 1,
      scope: 'windows-native-tauri-invoke-observer',
      runId: typeof runId === 'string' ? runId : null,
      valid,
      total: valid ? observedCount : null,
      observedCount,
      invalidReasons: [...invalidReasons],
      installedAtMs,
      timeOriginMs,
      maxEvents,
      commands: commands.map((event) => ({ ...event })),
    };
  }
  Object.defineProperty(window, key, {
    value: Object.freeze({ snapshot, invalidate }),
  });
})();
