import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/global.css';
import './lib/i18n'; // Initialize i18next

// Apply system theme immediately (before React renders) so the login page
// already has the correct dark/light colors via CSS [data-theme] selectors.
(function applyInitialTheme() {
  const root = document.documentElement;
  // Use localStorage first (from previous session), fall back to system preference
  const saved = localStorage.getItem('i18nextLng');
  // Check system theme
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  root.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
  // Also set accent to default ocean (user's saved accent loads after login)
  root.style.setProperty('--accent-primary', '#5B7C99');
})();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
