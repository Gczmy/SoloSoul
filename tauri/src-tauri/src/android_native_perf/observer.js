// RF-312: Android transport send attempts only; never read request/response bodies.
(() => {
  'use strict';
  const key = '__SOLOSOUL_ANDROID_NATIVE_PERF__';
  if (window[key]) {
    window[key].invalidate('duplicate-installation');
    return;
  }
  const maxEvents = 16384;
  const maxCommandLength = 128;
  const commands = [];
  const reasons = new Set();
  const timeOriginMs = performance.timeOrigin;
  const installedAtMs = performance.now();
  const originalBridge = window.ipc;
  const originalPost = originalBridge?.postMessage;
  const bridgeDescriptor = Object.getOwnPropertyDescriptor(window, 'ipc');
  let bridgeFacade = null;
  const originalFetch = window.fetch;
  const originalInternals = window.__TAURI_INTERNALS__;
  const postDescriptor =
    originalBridge && Object.getOwnPropertyDescriptor(originalBridge, 'postMessage');
  const fetchDescriptor = Object.getOwnPropertyDescriptor(window, 'fetch');
  let runId = null;
  let observedCount = 0;
  let lastAtMs = installedAtMs;
  let bridgeInstalled = false;
  let fetchInstalled = false;

  function invalidate(reason) {
    reasons.add(reason);
  }
  function publicCommand(value) {
    return typeof value === 'string' && /^[A-Za-z0-9_.:|/-]{1,128}$/.test(value);
  }
  // Tauri 2.11.2 serializes this envelope with cmd first. Stop at that field's
  // closing quote; never scan or JSON.parse the payload, options or invoke key.
  function bridgeCommand(data) {
    const prefix = '{"cmd":"';
    if (typeof data !== 'string' || !data.startsWith(prefix)) return null;
    let command = '';
    for (let i = prefix.length; i < data.length && i <= prefix.length + maxCommandLength; i++) {
      const character = data[i];
      if (character === '"') return publicCommand(command) ? command : null;
      if (!/[A-Za-z0-9_.:|/-]/.test(character)) return null;
      command += character;
    }
    return null;
  }
  function record(command, transport) {
    observedCount += 1;
    const atMs = performance.now();
    if (!Number.isFinite(atMs) || atMs < lastAtMs) invalidate('invalid-event-clock');
    lastAtMs = atMs;
    const safeCommand = publicCommand(command) ? command : null;
    if (safeCommand === null) invalidate('unobservable-command');
    if (commands.length < maxEvents) commands.push({ command: safeCommand, transport, atMs });
    else invalidate('event-overflow');
  }
  function observedPost(...args) {
    try {
      record(bridgeCommand(args[0]), 'android-java-bridge');
    } catch {
      invalidate('bridge-observation-error');
    }
    return Reflect.apply(originalPost, this === bridgeFacade ? originalBridge : this, args);
  }
  function observedFetch(...args) {
    try {
      const input = args[0];
      const options = args[1];
      const text = typeof input === 'string' || input instanceof URL ? String(input) : input?.url;
      const url = new URL(text, window.location.href);
      if (url.hostname === 'ipc.localhost') {
        const methodDescriptor =
          options == null ? undefined : Object.getOwnPropertyDescriptor(options, 'method');
        let method = 'GET';
        if (
          methodDescriptor &&
          ('get' in methodDescriptor || typeof methodDescriptor.value !== 'string')
        ) {
          invalidate('unobservable-fetch-method');
        } else {
          if (methodDescriptor) method = methodDescriptor.value.toUpperCase();
          if (method === 'POST') {
            if (url.protocol !== 'http:' || url.port || url.username || url.password)
              invalidate('unsupported-ipc-origin');
            record(decodeURIComponent(url.pathname.slice(1)), 'android-channel-fetch');
          }
        }
      }
    } catch {
      invalidate('fetch-observation-error');
    }
    return Reflect.apply(originalFetch, this, args);
  }
  // Android Java host methods can ignore JS property replacement despite
  // writable/configurable descriptors. Keep that host untouched and place a
  // delegating object in its mutable window alias. Native calls retain the
  // original Java receiver, arguments, return value and thrown error.
  if (
    typeof originalPost === 'function' &&
    bridgeDescriptor &&
    'value' in bridgeDescriptor &&
    (bridgeDescriptor.writable || bridgeDescriptor.configurable)
  ) {
    try {
      bridgeFacade = Object.create(originalBridge);
      Object.defineProperty(bridgeFacade, 'postMessage', {
        ...(postDescriptor || {}),
        value: observedPost,
      });
      Object.defineProperty(window, 'ipc', { ...bridgeDescriptor, value: bridgeFacade });
      bridgeInstalled = window.ipc === bridgeFacade && bridgeFacade.postMessage === observedPost;
    } catch {
      invalidate('bridge-wrapper-installation-failed');
    }
  }
  if (!bridgeInstalled) invalidate('bridge-wrapper-unavailable');
  if (
    typeof originalFetch === 'function' &&
    fetchDescriptor &&
    'value' in fetchDescriptor &&
    (fetchDescriptor.writable || fetchDescriptor.configurable)
  ) {
    try {
      Object.defineProperty(window, 'fetch', { ...fetchDescriptor, value: observedFetch });
      fetchInstalled = window.fetch === observedFetch;
    } catch {
      invalidate('fetch-wrapper-installation-failed');
    }
  }
  if (!fetchInstalled) invalidate('fetch-wrapper-unavailable');
  if (!Number.isFinite(timeOriginMs) || !Number.isFinite(installedAtMs))
    invalidate('invalid-document-clock');
  function bind(value) {
    if (typeof value !== 'string' || !/^[a-f0-9]{32}$/.test(value))
      throw new Error('Invalid measurement identity');
    if (runId !== null && runId !== value)
      throw new Error('Document already bound to another measurement');
    runId = value;
    return snapshot();
  }
  function snapshot() {
    if (window.top !== window || document.querySelectorAll('iframe,frame').length !== 0)
      invalidate('nonexclusive-main-frame');
    if (performance.timeOrigin !== timeOriginMs) invalidate('document-clock-changed');
    if (window.ipc !== bridgeFacade || bridgeFacade?.postMessage !== observedPost)
      invalidate('bridge-wrapper-replaced');
    if (window.fetch !== observedFetch) invalidate('fetch-wrapper-replaced');
    const invokeDescriptor =
      originalInternals && Object.getOwnPropertyDescriptor(originalInternals, 'invoke');
    if (
      window.__TAURI_INTERNALS__ !== originalInternals ||
      typeof invokeDescriptor?.value !== 'function' ||
      invokeDescriptor.writable !== false ||
      invokeDescriptor.configurable !== false
    ) {
      invalidate('internal-invoke-unprotected');
    }
    return {
      schemaVersion: 1,
      scope: 'android-native-ipc-transport-observer',
      runId,
      valid: runId !== null && reasons.size === 0,
      invalidReasons: [...reasons],
      installedAtMs,
      timeOriginMs,
      atMs: performance.now(),
      maxEvents,
      observedCount,
      commands: commands.map((row) => ({ ...row })),
    };
  }
  Object.defineProperty(window, key, {
    value: Object.freeze({
      bind,
      snapshot,
      invalidate: () => invalidate('duplicate-installation'),
    }),
  });
})();
