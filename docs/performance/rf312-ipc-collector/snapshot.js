(() => {
  const observer = window.__SOLOSOUL_ANDROID_NATIVE_PERF__;
  const descriptor = Object.getOwnPropertyDescriptor(window.__TAURI_INTERNALS__ || {}, 'invoke');
  const readonly = !!descriptor && descriptor.writable === false && descriptor.configurable === false;
  const platform = document.documentElement.dataset.platform;
  const mainFrame = window.top === window;
  const frames = document.querySelectorAll('iframe,frame').length;
  // Calling snapshot before framework initialization would permanently invalidate
  // the observer. Admit only the actual ready production document.
  const ready = !!observer && readonly && platform === 'android' && mainFrame && frames === 0;
  return JSON.stringify({present: !!observer, platform, mainFrame, frames, invokeReadonly: readonly,
    observer: ready ? (__BIND_FIRST__ ? observer.bind('__RUN_ID__') : observer.snapshot()) : null});
})();
