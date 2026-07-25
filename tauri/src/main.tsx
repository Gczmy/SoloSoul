import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/tokens.css';
import './styles/global.css';
import './styles/themes.css';
import { initI18n } from './lib/i18n';
import { initPlatform } from '@/lib/platform';
import { logger } from '@/lib/logger';
// 预加载平台信息，供 isMobilePlatformSync 等同步判定使用（非阻塞）
initPlatform().catch((err) => logger.warn('[main] Platform init failed:', err));

// 移动端启动性能基线：记录应用启动时刻（MOB-P1-07）
const appStartTime = performance.now();
(window as typeof window & { __SOLOSOUL_APP_START_TIME?: number }).__SOLOSOUL_APP_START_TIME =
  appStartTime;

const rootEl = document.getElementById('root');

// Block initial render until i18n (system language detection via Rust) and
// UI prefs are loaded — by the time login page shows, the correct language,
// theme and accent are already applied (~1ms IPC read).
// i18n must init first so settingsStore's lazy changeLanguage doesn't race.
initI18n()
  .then(() => initPlatform())
  .then(() =>
    import('@/stores/settingsStore')
      .then((m) => m.useSettingsStore.getState())
      .then((store) => store.loadUiPreferences()),
  )
  .then(() => {
    ReactDOM.createRoot(rootEl!).render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    );
  });
