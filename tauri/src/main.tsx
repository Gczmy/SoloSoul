import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/global.css';
import './lib/i18n'; // Initialize i18next

// Apply saved UI preferences before React renders so the login page
// already has the correct theme, accent color, and language (§4.1).
import { useSettingsStore } from '@/stores/settingsStore';
useSettingsStore.getState().loadUiPreferences();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
