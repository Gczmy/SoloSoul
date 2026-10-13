import { build, normalizePath } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const root = fileURLToPath(new URL('../', import.meta.url));
const normalizedRoot = normalizePath(resolve(root));
const fixture = resolve(root, 'native-regression/windows-shell');
const destination = process.argv[2];
if (!destination || process.argv.length !== 3)
  throw new Error('Expected a new fixture HTML destination');
const aliases = [
  ...['authStore', 'settingsStore', 'syncStore'].map((name) => ({
    find: `@/stores/${name}`,
    replacement: resolve(fixture, 'stores.ts'),
  })),
  { find: '@/lib/ipcClient', replacement: resolve(fixture, 'bridge.ts') },
  { find: '@/lib/platform', replacement: resolve(fixture, 'platform.ts') },
  { find: '@/lib/i18n', replacement: resolve(fixture, 'i18n.ts') },
  ...['PrimaryNavZone', 'SecondaryActionBar', 'TopFunctionBar', 'MobileBottomNav'].map((name) => ({
    find: `./${name}`,
    replacement: resolve(fixture, 'slots.tsx'),
  })),
  { find: '@/components/android/AndroidNavigation', replacement: resolve(fixture, 'slots.tsx') },
  { find: '@/components/sync/PairingDialog', replacement: resolve(fixture, 'slots.tsx') },
  { find: '@/hooks/useAndroidGlass', replacement: resolve(fixture, 'slots.tsx') },
  { find: '@', replacement: resolve(root, 'src') },
];
const result = await build({
  configFile: false,
  root,
  plugins: [react()],
  resolve: { alias: aliases },
  build: {
    write: false,
    cssCodeSplit: false,
    assetsInlineLimit: Infinity,
    rolldownOptions: {
      input: resolve(fixture, 'main.tsx'),
      output: { format: 'iife', codeSplitting: false },
    },
  },
});
const chunks = result.output.filter((item) => item.type === 'chunk');
// Rolldown retains an inlined self edge in chunk metadata. Reject real asset
// references and any edge to another chunk, rather than treating that metadata
// as a request. Declarative MemoryRouter routes have no lazy/module loaders.
const chunk = chunks[0];
if (
  chunks.length !== 1 ||
  chunk.imports.length ||
  chunk.dynamicImports.some((id) => id !== chunk.fileName) ||
  chunk.code.includes(chunk.fileName.split('/').at(-1))
)
  throw new Error('Expected one self-contained chunk');
const modules = Object.keys(chunks[0].modules)
  .map((id) => normalizePath(id.split('?')[0]))
  .filter((id) => id.startsWith(normalizedRoot + '/') && !id.includes('/node_modules/'));
if (
  modules.some((id) =>
    /\/src\/(App\/|pages\/auth\/|stores\/(authStore|settingsStore|syncStore)\.ts)/.test(id),
  )
)
  throw new Error('Production authentication/setup entered the isolated fixture');
for (const file of [
  'lib/nativeWindow.ts',
  'lib/nativeTitlebarControls.ts',
  'components/layout/AppBar.tsx',
  'components/layout/AppShell.tsx',
  'components/layout/SideNavigation.tsx',
  'components/layout/AuthLayout.tsx',
  'components/attachment/PreviewTitlebar.tsx',
])
  if (!modules.includes(normalizePath(resolve(root, 'src', file))))
    throw new Error(`Missing production component: ${file}`);
const manifest = {};
for (const path of modules)
  manifest[path.slice(normalizedRoot.length + 1)] = createHash('sha256')
    .update(await readFile(path))
    .digest('hex');
const css = result.output
  .filter((item) => item.type === 'asset' && item.fileName.endsWith('.css'))
  .map((item) => item.source)
  .join('\n');
const html =
  '<!-- SoloSoul RF-1144 isolated public fixture -->\n<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>' +
  css.replaceAll('</style', '<\\/style') +
  '</style></head><body><div id="root"></div><script type="module">' +
  chunks[0].code.replaceAll('</script', '<\\/script') +
  '</script></body></html>';
await writeFile(destination, html, { flag: 'wx' });
await writeFile(destination + '.sources.json', JSON.stringify(manifest, null, 2), { flag: 'wx' });
console.log(
  `Fixture: ${destination}; ${Buffer.byteLength(html)} bytes; ${modules.length} source modules`,
);
