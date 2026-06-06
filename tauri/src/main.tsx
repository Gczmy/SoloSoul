import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/global.css';
import './lib/i18n'; // Initialize i18next

// Synchronously apply cached UI preferences before React renders.
// The cache is populated by settingsStore.loadUiPreferences() after first IPC fetch.
(function applyCachedTheme() {
  try {
    const raw = localStorage.getItem('solosoul_ui_prefs');
    if (!raw) return;
    const prefs = JSON.parse(raw);
    if (prefs.theme) {
      document.documentElement.setAttribute('data-theme',
        prefs.theme === 'dark' ? 'dark' : prefs.theme === 'light' ? 'light' : 'system');
    }
    if (prefs.accentColor) {
      const colors: Record<string, string> = { ocean: '#5B7C99', amber: '#C4925C', forest: '#5B8C6F', rose: '#B06B7A' };
      document.documentElement.style.setProperty('--accent-primary', colors[prefs.accentColor] || '#5B7C99');
    }
  } catch { /* ignore corrupt cache */ }
})();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
