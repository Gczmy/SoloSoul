import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/tokens.css';
import './styles/global.css';
import './styles/themes.css';
import { initI18n } from './lib/i18n';
import { initLlmNotificationListener } from '@/lib/notification';

// Start global LLM notification listener (non-blocking)
initLlmNotificationListener().catch(() => {});

const rootEl = document.getElementById('root');

// Block initial render until i18n (system language detection via Rust) and
// UI prefs are loaded — by the time login page shows, the correct language,
// theme and accent are already applied (~1ms IPC read).
// i18n must init first so settingsStore's lazy changeLanguage doesn't race.
initI18n().then(() =>
  import('@/stores/settingsStore').then(m =>
    m.useSettingsStore.getState()
  ).then(store => store.loadUiPreferences())
).then(() => {
  ReactDOM.createRoot(rootEl!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
