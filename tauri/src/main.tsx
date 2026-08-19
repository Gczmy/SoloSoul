import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/tokens.css';
import './styles/global.css';
import './styles/themes.css';
import './styles/animations.css';
import { initI18n } from './lib/i18n';
import { initPlatform } from '@/lib/platform';
import { preloadCameraCapability } from '@/lib/cameraCapability';
import { logger } from '@/lib/logger';
// 预加载平台信息，供 isMobilePlatformSync 等同步判定使用（非阻塞）
initPlatform().catch((err) => logger.warn('[main] Platform init failed:', err));
// 启动即检测设备摄像头能力（非阻塞、非侵入，不触发权限弹窗），
// 供「从其他设备恢复」流程自适应默认 tab（支持→扫码，不支持→手动输入）
preloadCameraCapability().catch((err) =>
  logger.warn('[main] Camera capability check failed:', err),
);
// 启动期预探测指纹/PIN 可用性（非阻塞）——登录页挂载时结果已就绪或更快返回，
// 配合 loginMethodCache（localStorage 持久化登录方式）消灭「先显示主密码再跳指纹」闪屏。
import('@/lib/loginAvailabilityPreflight')
  .then((m) => m.preflightForLastAccount())
  .catch((err) => logger.warn('[main] Login availability preflight failed:', err));

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
  })
  .catch((err) => {
    // P008: 启动链任一环节失败也不白屏——兜底渲染（i18n 内部已逐层兜底到
    // navigator.language，此处防御 initI18n 本身抛错等极端情况），错误落日志。
    logger.error('[main] App bootstrap failed, rendering with defaults:', err);
    ReactDOM.createRoot(rootEl!).render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    );
  });
