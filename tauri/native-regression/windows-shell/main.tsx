import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18next from 'i18next';
import common from '@/locales/en-US/common.json';
import navigation from '@/locales/en-US/navigation.json';
import settings from '@/locales/en-US/settings.json';
import { AppShell } from '@/components/layout/AppShell';
import { AuthLayout } from '@/components/layout/AuthLayout';
import { ShellNotificationsProvider } from '@/components/layout/ShellNotifications';
import { PreviewWindow } from '@/components/attachment/PreviewWindow';
import { PreviewTitlebar } from '@/components/attachment/PreviewTitlebar';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import {
  prepareStartupWindow,
  observeNativeWindowLayout,
  refreshNativeAppearance,
} from '@/lib/nativeWindow';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { useUiStore } from '@/stores/uiStore';
import { invokeCommand } from './bridge';
import { useSettingsStore } from './stores';
import '@/styles/tokens.css';
import '@/styles/global.css';
import '@/styles/themes.css';
import '@/styles/action-buttons.css';
import '@/styles/icon-buttons.css';
import '@/styles/choice-controls.css';
import '@/styles/switch-controls.css';
import '@/styles/checkbox-controls.css';
import '@/styles/animations.css';
import '@/styles/android.css';
import '@/styles/macos-glass.css';
import '@/styles/desktop-controls.css';
import '@/styles/windows-material.css';
import '@/styles/startup.css';

function bounds(selector: string) {
  return Array.from(document.querySelectorAll<HTMLElement>(selector)).map((element) => {
    const r = element.getBoundingClientRect();
    return {
      label: element.getAttribute('aria-label') ?? element.textContent?.slice(0, 80),
      x: r.x,
      y: r.y,
      width: r.width,
      height: r.height,
    };
  });
}
async function capture() {
  const frame = useNativeWindowStore.getState().windowsFrame;
  const frontend = {
    theme: document.documentElement.dataset.theme,
    material: document.documentElement.dataset.nativeMaterial,
    highContrast: document.documentElement.dataset.highContrast,
    surfaces: Object.fromEntries(
      ['html', 'body', '[data-appbar]', '#desktop-navigation', '[data-shell-surface]'].map(
        (selector) => {
          const element = document.querySelector(selector);
          return [selector, element ? getComputedStyle(element).backgroundColor : null];
        },
      ),
    ),
    route: document.querySelector('[data-fixture-route]')?.getAttribute('data-fixture-route'),
    frame,
    sidebarExpanded: useUiStore.getState().sidebarExpanded,
    header: bounds('[data-appbar], [data-windows-titlebar], [data-preview-titlebar]'),
    controls: bounds(
      '[data-appbar] button, [data-windows-titlebar] button, [data-preview-titlebar] button',
    ),
    content: bounds('[data-shell-surface], [data-shell-content], [data-auth-content]'),
    navigation: bounds('#desktop-navigation'),
    primary: bounds('#desktop-navigation [class*="navPrimary"]'),
    addPage: bounds('[data-add-page-zone]'),
    tools: bounds('[data-sidebar-tools]'),
    draft: (document.querySelector('[data-public-draft]') as HTMLInputElement)?.value ?? null,
  };
  const native = await invokeCommand('frame_action', {
    action: 'probe',
    enabled: Boolean(frame),
    dark: document.documentElement.dataset.theme === 'dark',
    clicks: Number(document.querySelector('[data-interact]')?.getAttribute('data-interact') ?? 0),
    regions: [],
  });
  await invokeCommand('frame_frontend_report', { report: { native, frontend } });
}
function Fixture() {
  const [clicks, setClicks] = useState(0);
  const [preview, setPreview] = useState(false);
  const [detail, setDetail] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  useEffect(observeNativeWindowLayout, []);
  useEffect(() => {
    const timer = setTimeout(() => void capture(), 250);
    const probe = (event: KeyboardEvent) => {
      if (event.key === 'F8') void capture();
    };
    window.addEventListener('keydown', probe);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('keydown', probe);
    };
  }, [location.key, clicks, preview, detail]);
  const theme = async () => {
    document.documentElement.dataset.theme =
      document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    await refreshNativeAppearance();
    await capture();
  };
  const primary = (
    <>
      <Button data-interact={clicks} onClick={() => setClicks((value) => value + 1)}>
        Interact {clicks}
      </Button>
      <Button variant="secondary" onClick={() => setDetail(true)}>
        Object detail
      </Button>
    </>
  );
  const actions = (
    <>
      <Button variant="secondary" onClick={() => void theme()}>
        Light / Dark
      </Button>
      <Button variant="secondary" onClick={() => setPreview(true)}>
        Preview
      </Button>
      <Button variant="secondary" onClick={() => navigate('/auth')}>
        Authentication layout
      </Button>
      <Button variant="secondary" onClick={() => void capture()}>
        Capture layout
      </Button>
      <Button
        variant="secondary"
        onClick={() =>
          useSettingsStore.setState({
            settings: { ...useSettingsStore.getState().settings, sidebarPosition: 'right' },
          })
        }
      >
        Right navigation
      </Button>
    </>
  );
  return (
    <div data-fixture-route={location.pathname}>
      <Routes>
        <Route path="/auth" element={<AuthLayout />}>
          <Route
            index
            element={
              <div style={{ height: '100%', display: 'grid', placeItems: 'center' }}>
                <div style={{ background: 'var(--bg-elevated)', padding: 32, borderRadius: 16 }}>
                  <h1>Public authentication layout</h1>
                  <p>No account, password or Vault is loaded.</p>
                  <Button onClick={() => navigate('/')}>Return to workspace</Button>
                </div>
              </div>
            }
          />
        </Route>
        <Route
          path="*"
          element={
            <AppShell
              title={location.pathname === '/' ? 'Workspace' : 'Identity'}
              primaryActions={primary}
              actions={actions}
              onBack={location.pathname === '/' ? undefined : () => navigate('/')}
            >
              <h2>Windows shell regression</h2>
              <p>Production window chrome and full sidebar, synthetic business content.</p>
              <label>
                Public fixture draft <input data-public-draft defaultValue="public fixture edit" />
              </label>
              <div style={{ minHeight: 1200, paddingTop: 24 }}>
                <p>Scrollable content retains its own surface.</p>
              </div>
            </AppShell>
          }
        />
      </Routes>
      <Dialog isOpen={detail} onClose={() => setDetail(false)} title="Public object detail">
        {(close) => (
          <>
            <p>Public field: caption backdrop regression</p>
            <Button onClick={close}>Close detail</Button>
          </>
        )}
      </Dialog>
      {preview && (
        <PreviewWindow
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 2000,
            display: 'flex',
            flexDirection: 'column',
            background: 'var(--bg-base)',
          }}
        >
          <PreviewTitlebar>
            <Button onClick={() => setPreview(false)}>Close preview</Button>
            <span style={{ flex: 1 }}>Public attachment preview</span>
            <Button variant="secondary" onClick={() => void theme()}>
              Light / Dark
            </Button>
          </PreviewTitlebar>
          <div style={{ padding: 24 }}>
            <h1>Public preview content</h1>
            <p>This is an isolated layout fixture.</p>
          </div>
        </PreviewWindow>
      )}
    </div>
  );
}
async function bootstrap() {
  document.documentElement.dataset.theme = 'dark';
  document.documentElement.dataset.platform = 'windows';
  await i18next.init({
    lng: 'en-US',
    resources: { 'en-US': { common, navigation, settings } },
    interpolation: { escapeValue: false },
  });
  await prepareStartupWindow();
  useUiStore.setState({ sidebarExpanded: true });

  createRoot(document.getElementById('root')!).render(
    <I18nextProvider i18n={i18next}>
      <MemoryRouter>
        <ShellNotificationsProvider notifications={null}>
          <Fixture />
        </ShellNotificationsProvider>
      </MemoryRouter>
    </I18nextProvider>,
  );
}
void bootstrap();
