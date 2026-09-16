import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/tokens.css';
import './styles/global.css';
import './styles/themes.css';
import './styles/animations.css';
import './styles/android.css';
import './styles/macos-glass.css';
import { initI18n } from './lib/i18n';
import { initPlatform } from '@/lib/platform';
import { preloadCameraCapability } from '@/lib/cameraCapability';
import { logger } from '@/lib/logger';
import { useSettingsStore } from '@/stores/settingsStore';

/** 重模块求值期间，独立启动层已提供品牌首帧和恢复入口。 */
export async function mountApplication(): Promise<void> {
  if (window.__SOLOSOUL_STARTUP__?.active() === false) return;
  window.__SOLOSOUL_STARTUP__?.phase('preferences');
  preloadCameraCapability().catch((err) =>
    logger.warn('[main] Camera capability check failed:', err),
  );
  void import('@/lib/loginAvailabilityPreflight')
    .then((m) => m.preflightForLastAccount())
    .catch((err) => logger.warn('[main] Login availability preflight failed:', err));
  await initI18n();
  await initPlatform().catch((err) => logger.warn('[main] Platform init failed:', err));
  await useSettingsStore.getState().loadUiPreferences();
  document.documentElement.dataset.userReduceMotion = String(
    useSettingsStore.getState().settings.reduceMotion,
  );
  document.documentElement.dataset.androidGlass = useSettingsStore.getState().settings.androidGlass;
  if (window.__SOLOSOUL_STARTUP__?.active() === false) return;
  window.__SOLOSOUL_STARTUP__?.phase('accounts');
  performance.mark('solosoul:react-mount');
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
