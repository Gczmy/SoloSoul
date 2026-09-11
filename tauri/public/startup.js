/* 独立于应用模块：即使 React 下载失败，也能提供启动状态与恢复入口。 */
(function () {
  var root = document.documentElement;
  var started = performance.now();
  window.__SOLOSOUL_APP_START_TIME = started;
  var language = (navigator.language || 'en').startsWith('zh') ? 'zh-CN' : 'en-US';
  var mode = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  try {
    var storedLanguage = localStorage.getItem('i18nextLng');
    if (storedLanguage === 'zh-CN' || storedLanguage === 'en-US') language = storedLanguage;
    var prefs = JSON.parse(localStorage.getItem('solosoul_ui_prefs') || '{}');
    if (prefs.theme === 'light' || prefs.theme === 'dark') mode = prefs.theme;
    var snapshot = prefs.startupTheme;
    if (snapshot && snapshot.mode === mode) {
      ['background', 'foreground', 'secondary'].forEach(function (key) {
        if (/^#[\da-f]{6}$/i.test(snapshot[key])) {
          root.style.setProperty('--startup-' + key, snapshot[key]);
        }
      });
    }
  } catch (_) {
    /* 缓存不可用不阻断启动。 */
  }
  root.dataset.theme = mode;
  root.lang = language;
  var zh = language === 'zh-CN';
  var state = 'loading';
  var phase = 'application';
  var reason = '';
  var slow = false;
  var elapsed = 0;
  function render() {
    var screen = document.getElementById('startup-screen');
    if (!screen) return;
    screen.dataset.state = state;
    screen.querySelector('.startup-status').textContent =
      state === 'error'
        ? zh
          ? '启动未完成，请重试。'
          : 'Startup could not finish. Please retry.'
        : slow && state === 'loading'
          ? zh
            ? '正在准备 SoloSoul…'
            : 'Preparing SoloSoul…'
          : '';
    screen.querySelector('.startup-actions').hidden = state !== 'error';
    screen.querySelector('[data-startup-retry]').textContent = zh ? '重新启动' : 'Restart';
    screen.querySelector('[data-startup-diagnostics]').textContent = zh
      ? '查看诊断'
      : 'Diagnostics';
    screen.querySelector('.startup-diagnostics').textContent =
      'SoloSoul startup\nphase: ' + phase + '\nreason: ' + reason + '\nelapsed: ' + elapsed + ' ms';
  }
  var slowTimer = setTimeout(function () {
    slow = true;
    render();
  }, 1500);
  var timeout = setTimeout(function () {
    fail('timeout');
  }, 8000);
  function clearTimers() {
    clearTimeout(slowTimer);
    clearTimeout(timeout);
  }
  function fail(code) {
    if (state !== 'loading') return;
    state = 'error';
    reason = code;
    elapsed = Math.round(performance.now() - started);
    clearTimers();
    render();
  }
  window.__SOLOSOUL_STARTUP__ = {
    phase: function (next) {
      if (state === 'loading') phase = next;
    },
    active: function () {
      return state === 'loading';
    },
    fail: fail,
    ready: function () {
      if (state === 'error') return false;
      state = 'ready';
      clearTimers();
      render();
      return true;
    },
  };
  document.addEventListener(
    'DOMContentLoaded',
    function () {
      render();
      document.querySelector('[data-startup-retry]').addEventListener('click', function () {
        // 重新加载隔离上一轮尚未返回的 IPC，避免迟到结果覆盖新一轮状态。
        location.reload();
      });
      document.querySelector('[data-startup-diagnostics]').addEventListener('click', function () {
        var details = document.querySelector('.startup-diagnostics');
        details.hidden = !details.hidden;
      });
    },
    { once: true },
  );
})();
