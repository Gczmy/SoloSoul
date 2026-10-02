// 独立原生验收入口：只载入生产 Card 和样式，不载入应用、账户或插件初始化。
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { Card } from '../../src/components/ui/Card';
import { CardGrid } from '../../src/components/ui/CardGrid';
import { applyAndroidMaterial } from '../../src/lib/androidMaterial';
import '../../src/styles/tokens.css';
import '../../src/styles/global.css';
import '../../src/styles/themes.css';
import '../../src/styles/android.css';
import '../../src/styles/macos-glass.css';
import '../../src/styles/windows-material.css';
import './fixture.css';

declare global {
  interface Window {
    AndroidCardFixture?: { report: (report: string) => void };
    __runCardSample: (
      theme: 'light' | 'dark',
      appearance: {
        material: string;
        reduceMotion: boolean;
        highContrast: boolean;
      },
      platform?: 'macos' | 'android',
    ) => Promise<void>;
  }
}

async function reportSurface(report: unknown) {
  if (window.AndroidCardFixture) window.AndroidCardFixture.report(JSON.stringify(report));
  else await invoke('card_surface_report', { report });
}

flushSync(() =>
  createRoot(document.getElementById('root')!).render(
    <main className="fixture">
      <h1>RF-121 · 原生 Card 验收</h1>
      <p>合成内容，无账户数据。仅覆盖本轮已记录的平台和辅助功能场景。</p>
      <CardGrid>
        <Card>
          <h2>正文卡片</h2>
          <p>设备名称、模板与字段内容</p>
          <p>中文长内容验证：{'合成字段内容'.repeat(30)}</p>
        </Card>
        <Card surface="floating">
          <h2>浮动卡片</h2>
          <p>保留清晰的正文表面</p>
          <p>English long content: {'synthetic attachment metadata '.repeat(20)}</p>
        </Card>
      </CardGrid>
      <output id="result">等待原生检查</output>
    </main>,
  ),
);

window.__runCardSample = async (theme, appearance, platform = 'macos') => {
  const root = document.documentElement;
  root.dataset.platform = platform;
  if (platform === 'macos') root.dataset.desktopPlatform = 'macos';
  else delete root.dataset.desktopPlatform;
  root.dataset.theme = theme;
  if (platform === 'android') applyAndroidMaterial('ocean');
  root.dataset.nativeMaterial = appearance.material;
  root.dataset.reduceMotion = String(appearance.reduceMotion);
  root.dataset.highContrast = String(appearance.highContrast);
  // 等待主题过渡完成，避免把过渡中的颜色当作最终表面。
  // 后台 WebView 的定时器与动画时钟不同步；只在真实绘制帧中采样。
  let frameCount = 0;
  const watchdog = setTimeout(() => {
    void reportSurface({
      error: 'paint-frame-timeout',
      theme,
      frameCount,
      visibility: document.visibilityState,
      focused: document.hasFocus(),
    });
  }, 2500);
  await new Promise<void>((resolve) => {
    let started: number | undefined;
    const frame = (timestamp: number) => {
      frameCount += 1;
      started ??= timestamp;
      if (timestamp - started >= 300) resolve();
      else requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
  clearTimeout(watchdog);
  const probe = document.createElement('span');
  probe.style.backgroundColor = 'var(--bg-elevated)';
  root.append(probe);
  const expectedBackground = getComputedStyle(probe).backgroundColor;
  probe.remove();
  const cards = [...document.querySelectorAll<HTMLElement>('[data-ui-card]')].map((card) => {
    const style = getComputedStyle(card);
    const rect = card.getBoundingClientRect();
    const expected = document.createElement('span');
    expected.style.backgroundColor =
      platform === 'android' && card.dataset.uiCard === 'floating'
        ? 'var(--md-surface-high, var(--bg-inset))'
        : 'var(--bg-elevated)';
    root.append(expected);
    const expectedBackground = getComputedStyle(expected).backgroundColor;
    expected.remove();
    return {
      surface: card.dataset.uiCard,
      background: style.backgroundColor,
      expectedBackground,
      backdropFilter:
        style.getPropertyValue('backdrop-filter') ||
        style.getPropertyValue('-webkit-backdrop-filter'),
      color: style.color,
      width: rect.width,
      height: rect.height,
      overflow: card.scrollWidth > card.clientWidth + 1,
      visible: rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden',
    };
  });
  const report = {
    theme,
    appearance,
    expectedBackground,
    cards,
    pageOverflow: document.documentElement.scrollWidth > innerWidth + 1,
    viewport: { width: innerWidth, height: innerHeight },
  };
  document.getElementById('result')!.textContent = JSON.stringify(report, null, 2);
  await reportSurface(report);
};
