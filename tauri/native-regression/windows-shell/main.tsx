import React, { useState, useEffect, useSyncExternalStore } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18next from 'i18next';
import common from '@/locales/zh-CN/common.json';
import navigation from '@/locales/zh-CN/navigation.json';
import settings from '@/locales/zh-CN/settings.json';
import editor from '@/locales/zh-CN/editor.json';
import { AppShell } from '@/components/layout/AppShell';
import { ShellLayout } from '@/components/layout/ShellLayout';
import { ObjectEditorPage } from '@/pages/editor/ObjectEditorPage';
import { AuthLayout } from '@/components/layout/AuthLayout';
import { ShellNotificationsProvider } from '@/components/layout/ShellNotifications';
import { PreviewWindow } from '@/components/attachment/PreviewWindow';
import { PreviewTitlebar } from '@/components/attachment/PreviewTitlebar';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { prepareStartupWindow, observeNativeWindowLayout } from '@/lib/nativeWindow';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { useUiStore } from '@/stores/uiStore';
import { invokeCommand } from './bridge';
import { publicObjects, useSettingsStore } from './stores';
import { useMediaPreference } from '@/hooks/useMediaPreference';
import { followsSystemMotion } from '@/lib/motionPolicy';
import { applyTheme } from '@/lib/theme';
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
function subscribeNativeMotion(notify: () => void) {
  const observer = new MutationObserver(notify);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-reduce-motion', 'data-motion-policy'],
  });
  return () => observer.disconnect();
}
async function capture() {
  const frame = useNativeWindowStore.getState().windowsFrame;
  const frontend = {
    theme: document.documentElement.dataset.theme,
    material: document.documentElement.dataset.nativeMaterial,
    headerPaint: document.querySelector('[data-appbar]')
      ? getComputedStyle(document.querySelector('[data-appbar]')!).backgroundImage
      : null,
    highContrast: document.documentElement.dataset.highContrast,
    motion: {
      userReduced: useSettingsStore.getState().settings.reduceMotion,
      nativeReduced: document.documentElement.dataset.systemReduceMotion === 'true',
      systemReduced: window.matchMedia('(prefers-reduced-motion: reduce)').matches,
      policy: document.documentElement.dataset.motionPolicy,
    },
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
    editorToolbar: bounds('[data-editor-content-toolbar]'),
    editorToolbarControls: bounds('[data-editor-content-toolbar] button'),
    editorForm: bounds('[data-object-editor-form]'),
    editorScrollMask: document.querySelector('[data-shell-content]')
      ? getComputedStyle(document.querySelector('[data-shell-content]')!).maskImage
      : null,
    content: bounds('[data-shell-surface], [data-shell-content], [data-auth-content]'),
    navigation: bounds('#desktop-navigation'),
    navigationControls: bounds('#desktop-navigation button'),
    navigationIcons: bounds('#desktop-navigation button > svg'),
    pagePane: bounds('[data-page-pane], [data-page-pane-scroll]'),
    pagePaneWidth: useUiStore.getState().desktopPagePaneWidth,
    pagePaneOpen: useUiStore.getState().desktopPagePaneOpen,
    objectInspectorOpen: useUiStore.getState().desktopObjectInspectorOpen,
    objectInspectorTab: useUiStore.getState().desktopObjectInspectorTab,
    objectInspector: bounds('[data-object-inspector], [data-object-inspector-scroll]'),
    inspectorObjectId: document
      .querySelector('[data-object-inspector]')
      ?.getAttribute('data-object-id'),
    historyRows: bounds('[data-history-snapshot]'),
    attachmentRows: bounds('[data-inspector-attachment]'),
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
  const userReduced = useSettingsStore((state) => state.settings.reduceMotion);
  const systemReduced = useMediaPreference('(prefers-reduced-motion: reduce)');
  const nativeReduced = useSyncExternalStore(
    subscribeNativeMotion,
    () => document.documentElement.dataset.reduceMotion === 'true',
    () => false,
  );
  const systemLimited = followsSystemMotion() && (systemReduced || nativeReduced);
  const motionEnabled = !userReduced && !systemLimited;
  const navigate = useNavigate();
  const location = useLocation();
  const selectedObject = publicObjects.find(
    (object) => location.pathname === `/editor/${object.id}`,
  );
  useEffect(observeNativeWindowLayout, []);
  useEffect(() => {
    const timer = setTimeout(() => void capture(), 750);
    const probe = (event: KeyboardEvent) => {
      if (event.key === 'F8') void capture();
      if (event.key === 'F6') void theme();
    };
    window.addEventListener('keydown', probe);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('keydown', probe);
    };
  }, [location.key, clicks, preview, detail, userReduced, systemReduced, nativeReduced]);
  const theme = async () => {
    const dark = document.documentElement.dataset.theme !== 'dark';
    await applyTheme({ preset: dark ? 'warm-stone-dark' : 'warm-stone', accentColor: 'ocean' });
    await capture();
  };
  const primary = (
    <>
      <Button variant="secondary" onClick={() => setDetail(true)}>
        Object detail
      </Button>
    </>
  );
  const actions = (
    <>
      <Button
        variant="secondary"
        aria-pressed={motionEnabled}
        aria-describedby="fixture-motion-status"
        disabled={systemLimited}
        onClick={() =>
          useSettingsStore.setState((state) => ({
            settings: { ...state.settings, reduceMotion: !state.settings.reduceMotion },
          }))
        }
      >
        动态效果：{systemLimited ? '系统关闭' : userReduced ? '关闭' : '开启'}
      </Button>
      <Button variant="secondary" onClick={() => void theme()}>
        Light / Dark
      </Button>
      <Button variant="secondary" onClick={() => void capture()}>
        Capture layout
      </Button>
    </>
  );
  return (
    <div data-fixture-route={location.pathname}>
      <Routes>
        <Route element={<ShellLayout />}>
          <Route path="/editor/:objectId" element={<ObjectEditorPage />} />
        </Route>
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
              title={selectedObject ? '编辑公开示例对象' : '桌面布局验收'}
              primaryActions={primary}
              actions={actions}
              onBack={location.pathname === '/' ? undefined : () => navigate('/')}
            >
              <h2 style={{ fontSize: 16, lineHeight: '24px', fontWeight: 400 }}>
                {selectedObject?.name ?? '最新桌面布局 · Windows 原生验收'}
              </h2>
              <p id="fixture-motion-status" role="status">
                {systemLimited
                  ? '系统正在减少动态效果。请先在 Windows 中启用动画效果，再使用顶栏开关验收；本窗口遵循系统设置。'
                  : motionEnabled
                    ? '动态效果已开启：桌面动画独立于系统设置，顶栏开关可关闭动画；页面树展开、收起及长名称悬停滚动均使用正式实现。'
                    : '动态效果已关闭：点击顶栏“动态效果：关闭”开启动画，仅影响本次隔离验收。'}
              </p>
              <p>使用当前源码的正式窗框、顶栏、图标侧栏与页面树；业务内容为公开示例。</p>
              <p>
                可测试顶栏折叠、分隔线调宽、搜索、页面展开、对象跳转、新建页面卡片及长名称悬停滚动。
              </p>
              <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 16 }}>
                <Button data-interact={clicks} onClick={() => setClicks((value) => value + 1)}>
                  Interact {clicks}
                </Button>
                <Button variant="secondary" onClick={() => setPreview(true)}>
                  Preview
                </Button>
                <Button variant="secondary" onClick={() => navigate('/auth')}>
                  Authentication layout
                </Button>
              </div>
              <label>
                公开示例草稿{' '}
                <input
                  key={location.pathname}
                  data-public-draft
                  defaultValue={selectedObject?.name ?? 'public fixture edit'}
                />
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
  await applyTheme({ preset: 'warm-stone-dark', accentColor: 'ocean' });
  document.documentElement.dataset.platform = 'windows';
  await i18next.init({
    lng: 'zh-CN',
    resources: { 'zh-CN': { common, navigation, settings, editor } },
    interpolation: { escapeValue: false },
  });
  await prepareStartupWindow();
  useUiStore.setState({
    sidebarExpanded: false,
    desktopPagePaneOpen: true,
    desktopPagePaneWidth: 264,
    desktopObjectInspectorOpen: true,
    desktopObjectInspectorTab: 'history',
  });

  createRoot(document.getElementById('root')!).render(
    <I18nextProvider i18n={i18next}>
      <MemoryRouter initialEntries={['/', '/editor/public-object-0']} initialIndex={1}>
        <ShellNotificationsProvider notifications={null}>
          <Fixture />
        </ShellNotificationsProvider>
      </MemoryRouter>
    </I18nextProvider>,
  );
}
void bootstrap();
