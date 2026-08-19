#!/usr/bin/env node
// =============================================================================
// 回归守卫：micromark 家族必须与 micromark 核心同 chunk。
//
// 背景（帮助文档 `t[n].add` 崩溃根因）：vite.config.ts 的 markdown-vendor
// codeSplitting 组正则曾写作 `micromark[\\/]`，只匹配核心包 micromark，
// 漏掉 micromark-util-*/micromark-core-commonmark/micromark-extension-*——
// 它们留在 index chunk 后与 markdown-vendor 形成跨 chunk 循环依赖：
// vendor 内 micromark 求值时 index 尚未初始化完，constructs 数组塞入
// undefined，运行期 combineExtensions 报 `t[n].add`（帮助文档/关于页更新
// 内容全灭，与内容无关）。本脚本断言关键模块同 chunk，防止正则再次回归。
//
// 用法：node scripts/check-markdown-chunk-boundary.mjs
// =============================================================================
import { build } from 'vite';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const tauriRoot = path.join(root, '..');

// micromark 核心（markdown-vendor 组的既有成员）
const CORE = 'micromark/lib/parse.js';
// 曾因正则漏配而留在 index chunk、引发循环依赖的家族成员
const FAMILY = [
  'micromark-util-combine-extensions/',
  'micromark-util-chunked/',
  'micromark-core-commonmark/',
  'micromark-extension-gfm/',
  'micromark-extension-gfm-table/',
  'micromark-extension-gfm-task-list-item/',
];

const result = await build({
  configFile: path.join(tauriRoot, 'vite.config.ts'),
  logLevel: 'silent',
  build: { write: false, minify: false },
});

let coreChunk = null;
const memberChunks = new Map(); // module path -> chunk name

for (const c of result.output.filter((x) => x.type === 'chunk')) {
  for (const id of Object.keys(c.modules)) {
    if (!id.includes('/node_modules/')) continue;
    const nm = id.split('/node_modules/')[1];
    if (nm === CORE) coreChunk = c.name;
    for (const fam of FAMILY) {
      if (nm.startsWith(fam)) memberChunks.set(fam, c.name);
    }
  }
}

let failed = false;
if (!coreChunk) {
  console.error(`[check-markdown-chunk-boundary] FAIL: 未找到核心模块 ${CORE}（构建/路径变动？）`);
  failed = true;
}
for (const fam of FAMILY) {
  const chunk = memberChunks.get(fam);
  if (!chunk) {
    console.error(
      `[check-markdown-chunk-boundary] FAIL: ${fam} 未出现在任何 chunk（依赖被移除？）`,
    );
    failed = true;
  } else if (chunk !== coreChunk) {
    console.error(
      `[check-markdown-chunk-boundary] FAIL: ${fam} 在 ${chunk}，与 micromark 核心（${coreChunk}）不同 chunk —— ` +
        'markdown-vendor 正则漏配会复现帮助文档 `t[n].add` 崩溃，请检查 vite.config.ts 的 codeSplitting 组',
    );
    failed = true;
  }
}

if (failed) {
  process.exit(1);
}
console.log(`[check-markdown-chunk-boundary] OK: micromark 家族与核心同处 ${coreChunk}`);
