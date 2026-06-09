import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/tokens.css';
import './styles/global.css';
import './styles/themes.css';
import './lib/i18n'; // Initialize i18next
import { initLlmNotificationListener } from '@/lib/notification';

// Start global LLM notification listener (non-blocking)
initLlmNotificationListener().catch(() => {});

// Block initial render until UI prefs loaded — by the time login page shows,
// the correct theme and accent are already applied (~1ms IPC read).
const rootEl = document.getElementById('root');

// Load UI prefs synchronously before first render
const initPromise = import('@/stores/settingsStore').then(m =>
  m.useSettingsStore.getState()
).then(store => store.loadUiPreferences());

initPromise.then(() => {
  ReactDOM.createRoot(rootEl!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
