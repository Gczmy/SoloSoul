#!/usr/bin/env node
// 构建为单个 HTML；无服务器、账户数据、外部请求或正式应用启动代码。
import { build } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { writeFile } from 'node:fs/promises';

const root = fileURLToPath(new URL('../', import.meta.url));
const destination = process.argv[2];
if (!destination)
  throw new Error('用法：node scripts/build-card-surface-fixture.mjs /tmp/card-surfaces.html');
const result = await build({
  configFile: false,
  root,
  plugins: [react()],
  build: {
    write: false,
    cssCodeSplit: false,
    rolldownOptions: { input: resolve(root, 'native-regression/card-surfaces/main.tsx') },
  },
});
const chunks = result.output.filter((item) => item.type === 'chunk');
if (chunks.length !== 1 || chunks[0].imports.length || chunks[0].dynamicImports.length) {
  throw new Error('独立测试页必须只有一个内联 JS chunk');
}
// 防止后续组件引用链无意带入正式应用的账户、日志或插件初始化。
const allowed = new Set([
  'components/ui/Card.tsx',
  'components/ui/Card.module.css',
  'components/ui/CardGrid.tsx',
  'components/ui/CardGrid.module.css',
  'styles/tokens.css',
  'styles/global.css',
  'styles/themes.css',
  'styles/android.css',
  'styles/macos-glass.css',
  'styles/windows-material.css',
]);
const sourceModules = Object.keys(chunks[0].modules)
  .map((id) => id.split('?')[0])
  .filter((id) => id.startsWith(resolve(root, 'src') + '/'))
  .map((id) => id.slice(resolve(root, 'src').length + 1));
if (
  sourceModules.some((id) => !allowed.has(id)) ||
  !sourceModules.includes('components/ui/Card.tsx') ||
  !sourceModules.includes('components/ui/CardGrid.tsx')
) {
  throw new Error(`独立验收入口引用越界或缺少生产组件：${sourceModules.join(', ')}`);
}
const css = result.output.filter((item) => item.type === 'asset' && item.fileName.endsWith('.css'));
const html = `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>${css
  .map((item) => item.source)
  .join('\n')
  .replaceAll(
    '</style',
    '<\\/style',
  )}</style></head><body><div id="root"></div><script type="module">${chunks[0].code.replaceAll('</script', '<\\/script')}</script></body></html>`;
await writeFile(destination, html, { flag: 'wx' });
console.log(`Card fixture: ${destination}; ${Buffer.byteLength(html)} bytes`);
