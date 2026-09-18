import { defineConfig } from '@playwright/test';
import base from './playwright.config';

// 开发服务器不生成共享 chunk，无法发现仅在生产包中出现的循环初始化错误。
export default defineConfig({
  ...base,
  testMatch: [
    '**/production-startup.spec.ts',
    '**/native-theme.spec.ts',
    '**/*.production.spec.ts',
  ],
  testIgnore: [],
  outputDir: 'test-results/production',
  use: { ...base.use, baseURL: 'http://127.0.0.1:1423' },
  projects: base.projects?.filter((project) => project.name === 'chromium'),
  webServer: {
    command: 'npx vite build && npx vite preview --host 127.0.0.1 --port 1423 --strictPort',
    url: 'http://127.0.0.1:1423',
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
