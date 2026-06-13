/*
 * Browser init script that mocks Tauri v2 internals for Playwright E2E tests.
 * Must run before the React bundle loads so @tauri-apps/api modules pick up
 * window.__TAURI_INTERNALS__.
 */
(function () {
  if (typeof window === 'undefined') return;

  const callbacks = new Map();
  const eventListeners = new Map();
  let pluginRunCounter = 0;

  function registerCallback(callback, once = false) {
    const id = window.crypto.getRandomValues(new Uint32Array(1))[0];
    callbacks.set(id, (data) => {
      if (once) callbacks.delete(id);
      if (callback) return callback(data);
    });
    return id;
  }

  function runCallback(id, data) {
    const cb = callbacks.get(id);
    if (cb) cb(data);
  }

  function handleListen(args) {
    const list = eventListeners.get(args.event) || [];
    list.push(args.handler);
    eventListeners.set(args.event, list);
    return args.handler;
  }

  function handleEmit(args) {
    const list = eventListeners.get(args.event) || [];
    for (const handler of list) runCallback(handler, args.payload);
    return null;
  }

  function handleRemoveListener(args) {
    const list = eventListeners.get(args.event);
    if (!list) return;
    const idx = list.indexOf(args.id);
    if (idx !== -1) list.splice(idx, 1);
  }

  // Per-test command mocks can be injected through window.__E2E_MOCKS__.
  function userMocks() {
    return window.__E2E_MOCKS__ || {};
  }

  function resolveCommand(cmd, args) {
    const mocks = userMocks();
    if (mocks[cmd]) {
      return mocks[cmd](args);
    }
    return undefined;
  }

  async function invoke(cmd, args, _options) {
    if (window.__E2E_DEBUG__) {
      // eslint-disable-next-line no-console
      console.log('[E2E MOCK]', cmd, args);
    }
    if (cmd.startsWith('plugin:event|')) {
      switch (cmd) {
        case 'plugin:event|listen':
          return handleListen(args);
        case 'plugin:event|emit':
          return handleEmit(args);
        case 'plugin:event|unlisten':
          return handleRemoveListener(args);
      }
      return undefined;
    }

    // Default mocks to let the app boot and login.
    switch (cmd) {
      case 'ui_get_preferences':
        return { hasSeenOnboarding: true, theme: 'system', accentColor: 'ocean' };
      case 'list_accounts':
        return [{ id: 'e2e-account', name: 'E2E User' }];
      case 'check_has_account':
        return true;
      case 'login':
        return undefined;
      case 'profile_load':
        return null;
      case 'load_settings':
      case 'settings_load':
        return {
          theme: 'system',
          accentColor: 'ocean',
          customAccentHex: '',
          backgroundType: 'solid',
          backgroundValue: '',
          language: 'en',
          locale: 'en',
          autoLockTimeoutMinutes: 5,
          biometricEnabled: false,
          confirmDelete: true,
          customPages: [],
          defaultLightTheme: 'warm-stone',
          defaultDarkTheme: 'warm-stone-dark',
          sidebarPosition: 'left',
          sidebarBottomActions: ['search', 'plugins', 'ai_chat'],
        };
      case 'load_custom_pages':
      case 'custom_pages_load':
        return [];
      case 'biometric_check_availability':
        return { available: false };
      case 'set_titlebar_color':
        return undefined;
      case 'check_for_update':
        return null;
      case 'user_data_get_preferences':
        return {
          theme: 'system',
          accentColor: 'ocean',
          customAccentHex: '',
          backgroundType: 'solid',
          backgroundValue: '',
          language: 'en-US',
          locale: 'en-US',
          autoLockTimeoutMinutes: 5,
          biometricEnabled: false,
          confirmDelete: true,
          customPages: [],
          defaultLightTheme: 'warm-stone',
          defaultDarkTheme: 'warm-stone-dark',
          sidebarPosition: 'left',
          sidebarBottomActions: ['search', 'plugins', 'ai_chat'],
        };
      case 'object_list':
        return [];
      case 'plugin:window|set_size':
        return undefined;
      case 'plugin:window|inner_size':
        return { width: 1280, height: 800 };
      case 'plugin:webview|set_webview_background_color':
      case 'plugin:window|set_background_color':
        return undefined;
      case 'ui_update_preference':
      case 'user_data_update_preference':
        return undefined;
      case 'plugin_list_all':
        return userMocks()[cmd] ? userMocks()[cmd](args) : [];
      case 'plugin_list_installed':
        return userMocks()[cmd] ? userMocks()[cmd](args) : [];
      case 'plugin_install':
      case 'plugin_update':
      case 'plugin_uninstall':
      case 'plugin_consent_response':
      case 'plugin_dialog_response':
      case 'plugin_list_sessions':
      case 'plugin_update_registry':
        return userMocks()[cmd] ? userMocks()[cmd](args) : undefined;
      case 'plugin_run':
        pluginRunCounter += 1;
        let channelId = null;
        if (args.channel && typeof args.channel.id === 'number') {
          channelId = args.channel.id;
        } else if (typeof args.channel === 'string' && args.channel.startsWith('__CHANNEL__:')) {
          channelId = parseInt(args.channel.replace('__CHANNEL__:', ''), 10);
        } else if (typeof args.channel === 'number') {
          channelId = args.channel;
        }
        return new Promise((resolve) => {
          const handler = userMocks()['plugin_run'];
          if (handler) {
            let index = 0;
            const finish = (result) => resolve(result);
            const emit = (event) => {
              if (channelId != null) {
                runCallback(channelId, { message: event, index });
                index += 1;
              }
            };
            handler(args, emit, finish);
          } else {
            resolve({ exitCode: 0, logs: [], results: [], fuelConsumed: 0 });
          }
        });
      case 'plugin_audit_log':
        return userMocks()[cmd] ? userMocks()[cmd](args) : [];
      default:
        return resolveCommand(cmd, args);
    }
  }

  window.__TAURI_INTERNALS__ = {
    invoke,
    transformCallback: registerCallback,
    unregisterCallback: (id) => callbacks.delete(id),
    runCallback,
    callbacks,
    convertFileSrc: (filePath, protocol = 'asset') => `${protocol}://localhost/${encodeURIComponent(filePath)}`,
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { windowLabel: 'main', label: 'main' },
    },
  };

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (_event, id) => callbacks.delete(id),
  };

  // Mock WebviewWindow used by useWindowSize.
  const fakeWebviewWindow = {
    label: 'main',
    setSize: async () => {},
    onResized: async (cb) => {
      window.addEventListener('resize', () => cb({ payload: { width: window.innerWidth, height: window.innerHeight } }));
      return () => {};
    },
  };

  // @tauri-apps/api/webviewWindow calls getCurrentWebviewWindow which reads
  // __TAURI_INTERNALS__.metadata.currentWebview. The returned object must
  // expose setSize/onResized. We patch the module's expected prototype by
  // exposing a global helper that our bundle import resolves to.
  window.__TAURI_MOCK_WEBVIEW__ = fakeWebviewWindow;
})();
