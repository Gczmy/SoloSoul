const { defineConfig } = require('/Users/zzc/PycharmProjects/SoloSoul/tauri/node_modules/@playwright/test');
module.exports = defineConfig({
  testDir: '/Users/zzc/PycharmProjects/SoloSoul/tauri/e2e',
  testMatch: ['scroll-region.spec.ts'],
  outputDir: '/tmp/solosoul-css-hover-regression-20261008-results',
  workers: 2,
  retries: 0,
  reporter: [['list'], ['json', { outputFile: '/tmp/solosoul-css-hover-regression-20261008.json' }]],
  use: { baseURL: 'http://localhost:1420', viewport: { width: 1000, height: 700 } },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium', launchOptions: { ignoreDefaultArgs: ['--hide-scrollbars'] } } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
  webServer: { command: 'npm run dev', cwd: '/Users/zzc/PycharmProjects/SoloSoul/tauri', url: 'http://localhost:1420', reuseExistingServer: true, timeout: 120000 },
});
