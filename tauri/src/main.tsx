import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/global.css';
import './lib/i18n'; // Initialize i18next

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
