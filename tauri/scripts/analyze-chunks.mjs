import { build } from 'vite';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { readFileSync, existsSync } from 'node:fs';

const root = path.dirname(fileURLToPath(import.meta.url));
const tauriRoot = path.join(root, '..');

// minified sizes from the real build output (dist)
const distDir = path.join(tauriRoot, 'dist', 'assets');
const distSizes = {};
if (existsSync(distDir)) {
  const fs = await import('node:fs');
  for (const f of fs.readdirSync(distDir)) {
    if (f.endsWith('.js')) {
      const st = fs.statSync(path.join(distDir, f));
      distSizes[f] = Math.round(st.size / 1024);
    }
  }
}

const result = await build({
  configFile: path.join(tauriRoot, 'vite.config.ts'),
  logLevel: 'silent',
  build: { write: false, minify: false },
  plugins: [
    {
      name: 'chunk-analysis',
      generateBundle(_, bundle) {
        const chunks = Object.values(bundle).filter((c) => c.type === 'chunk');
        const byName = Object.fromEntries(chunks.map((c) => [c.name, c]));
        const byFile = Object.fromEntries(chunks.map((c) => [c.fileName, c.name]));
        const nameOf = (d) => byFile[d] || d;

        // 1) full chunk table: minified size (from dist), unminified size, package composition
        const rows = [];
        for (const c of chunks) {
          const byNodeModule = {};
          let appCode = 0;
          for (const id of Object.keys(c.modules)) {
            const m = c.modules[id];
            const bytes = m.originalLength ?? m.renderedLength ?? 0;
            const nm = id.includes('/node_modules/') ? id.split('/node_modules/')[1].split('/')[0] : null;
            if (nm) byNodeModule[nm] = (byNodeModule[nm] || 0) + bytes;
            else appCode += bytes;
          }
          const top = Object.entries(byNodeModule)
            .sort((a, b) => b[1] - a[1])
            .slice(0, 5)
            .map(([k, v]) => `${k}~${Math.round(v / 1024)}K`)
            .join(' ');
          const min = distSizes[c.fileName] ?? 0;
          rows.push({
            name: c.name,
            min,
            unmin: Math.round(c.code.length / 1024),
            app: Math.round(appCode / 1024),
            top,
            deps: c.imports.map((d) => nameOf(d)).join(','),
          });
        }
        rows.sort((a, b) => b.min - a.min);
        console.log('===== 全部 chunk（按压缩体积降序）=====');
        for (const r of rows) {
          console.log(
            `${String(r.min).padStart(5)}K min / ${String(r.unmin).padStart(5)}K raw  ${r.name.padEnd(24)} app:${r.app}K  ${r.top}`,
          );
        }

        // 2) startup chain from index.html modulepreload
        const html = readFileSync(path.join(tauriRoot, 'dist', 'index.html'), 'utf8');
        const preloads = [...html.matchAll(/assets\/([A-Za-z0-9_-]+\.js)/g)].map((m) => m[1]);
        const unique = [...new Set(preloads)];
        const inChain = (fn) => {
          const c = chunks.find((x) => x.fileName === fn);
          return c ? byName[c.name] : null;
        };
        console.log('\n===== 启动链（index.html modulepreload）=====');
        let startupTotal = 0;
        for (const fn of unique) {
          const min = distSizes[fn] ?? 0;
          startupTotal += min;
          const c = inChain(fn);
          const note = c && c.top ? `  [${c.top.slice(0, 80)}]` : '';
          console.log(`${String(min).padStart(5)}K  ${fn}${note}`);
        }
        console.log(`启动链 JS 合计：${startupTotal}K（min）`);

        // 3) static closure of a given root chunk (e.g. HomePage / PageContainer)
        const closure = (rootName) => {
          const visited = new Set();
          const queue = [rootName];
          const out = [];
          while (queue.length) {
            const n = queue.shift();
            if (!byName[n] || visited.has(n)) continue;
            visited.add(n);
            const r = rows.find((x) => x.name === n);
            if (r) out.push(r);
            for (const d of (byName[n].imports || [])) queue.push(nameOf(d));
          }
          return out;
        };
        for (const rootName of ['HomePage', 'PageContainer']) {
          console.log(`\n===== ${rootName} 静态闭包（含启动链外的增量）=====`);
          const extra = closure(rootName).filter((r) => !unique.some((f) => f.startsWith(r.name + '-')) && r.name !== 'index');
          extra.sort((a, b) => b.min - a.min);
          for (const r of extra) {
            console.log(`${String(r.min).padStart(5)}K min  ${r.name.padEnd(26)} ${r.top.slice(0, 78)}`);
          }
          const tot = extra.reduce((a, r) => a + r.min, 0);
          console.log(`  小计（启动链之外增量）：${tot}K min`);
        }

        // 4) who statically imports PageContainer (the biggest shared chunk)
        console.log('\n===== 静态引用 PageContainer 的 chunk =====');
        for (const c of chunks) {
          if ((c.imports || []).map((d) => nameOf(d)).includes('PageContainer')) {
            console.log(`${c.name}`);
          }
        }
      },
    },
  ],
});
