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
  // 只观察认证链；请求/响应body从不读取。独立有效性不改变旧IPC计数合同。
  const authReasons = new Set();
  const authReplies = [];
  const authFlows = [];
  const authStages = new Set([
    'started',
    'login-await-start',
    'login-await-ok',
    'login-await-error',
    'accounts-await-start',
    'accounts-await-ok',
    'accounts-await-error',
    'state-set-start',
    'state-set-done',
    'finished',
    'failed',
  ]);
  function beginAuthFlow() {
    if (authFlows.length >= 64) {
      authReasons.add('auth-flow-overflow');
      return null;
    }
    const id = authFlows.length + 1;
    authFlows.push({ id, events: [{ stage: 'started', atMs: performance.now() }] });
    return id;
  }
  function markAuthFlow(id, stage) {
    const flow = authFlows[id - 1];
    if (!Number.isInteger(id) || !flow || !authStages.has(stage) || stage === 'started') {
      authReasons.add('invalid-auth-checkpoint');
      return;
    }
    if (flow.events.length >= 16) {
      authReasons.add('auth-checkpoint-overflow');
      return;
    }
    flow.events.push({ stage, atMs: performance.now() });
  }
  function authSnapshot() {
    const counted = snapshot();
    return {
      schemaVersion: 1,
      scope: 'windows-native-auth-observer',
      runId,
      timeOriginMs,
      atMs: performance.now(),
      valid: counted.valid && authReasons.size === 0,
      invalidReasons: [...counted.invalidReasons, ...authReasons],
      invokeCount: counted.observedCount,
      maxReplies: 128,
      maxFlows: 64,
      replies: authReplies.map((entry) => ({ ...entry })),
      flows: authFlows.map((flow) => ({
        id: flow.id,
        events: flow.events.map((event) => ({ ...event })),
      })),
    };
  }
  function observeAuthReply(entry, promise) {
    if (!entry) return;
    try {
      if (typeof promise?.then !== 'function') throw Error();
      promise.then(
        (reply) => {
          try {
            const status = reply.headers.get('Tauri-Response');
            if (status !== 'ok' && status !== 'error') throw Error();
            entry.headersAtMs = performance.now();
            entry.status = status === 'ok' ? 'tauri-ok' : 'tauri-error';
          } catch {
            entry.status = 'unobservable';
            authReasons.add('auth-reply-unobservable');
          }
        },
        () => {
          entry.headersAtMs = performance.now();
          entry.status = 'transport-error';
        },
      );
    } catch {
      entry.status = 'unobservable';
      authReasons.add('auth-reply-unobservable');
    }
  }

  if (typeof runId !== 'string' || !/^[a-f0-9]{32}$/.test(runId)) {
    invalidReasons.add('missing-or-invalid-run-id');
  }
  function invalidate(reason) {
    invalidReasons.add(reason);
  }
  function observedFetch(...args) {
    let authEntry = null;
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
        if (command === 'login' || command === 'vault_list_accounts') {
          if (authReplies.length >= 128) authReasons.add('auth-reply-overflow');
          else {
            authEntry = {
              invokeSeq: observedCount,
              command,
              startedAtMs: performance.now(),
              headersAtMs: null,
              status: 'pending',
            };
            authReplies.push(authEntry);
          }
        }
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
    try {
      const returned = Reflect.apply(originalFetch, this, args);
      observeAuthReply(authEntry, returned);
      return returned;
    } catch (error) {
      if (authEntry) {
        authEntry.headersAtMs = performance.now();
        authEntry.status = 'sync-throw';
      }
      throw error;
    }
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
    value: Object.freeze({ snapshot, invalidate, beginAuthFlow, markAuthFlow, authSnapshot }),
  });
})();
