#!/usr/bin/env node
/**
 * 一次性 codemod：把 src 下写死的 fontSize 像素值替换为语义化 CSS token。
 *
 * 规则：
 * - 只处理 .tsx/.ts/.css（排除 *.test.* / *.spec.* / .d.ts / node_modules）
 * - 不碰图标 size={n}、动画字号、相对值（em/rem/%）
 * - TSX/TS 中：fontSize: <n>[, }] -> fontSize: 'var(--token)'
 * - CSS 中：font-size: <n>px; -> font-size: var(--token);
 */

import { readFile, writeFile, readdir, stat } from 'node:fs/promises';
import path from 'node:path';

const ROOT = path.resolve('src');

const TOKEN_MAP = {
  24: '--text-xl',
  20: '--text-page-title',
  18: '--text-md',
  17: '--text-md',
  16: '--text-section-title',
  15: '--text-card-title',
  14: '--text-body',
  13: '--text-body-sm',
  12: '--text-caption',
  11: '--text-badge',
  10: '--text-badge',
  9: '--text-badge',
};

const EXCLUDE_PATTERNS = [
  /node_modules/,
  /\.d\.ts$/,
  /\.(test|spec)\./,
];

function shouldProcess(filePath) {
  if (!/\.(tsx|ts|css)$/.test(filePath)) return false;
  return !EXCLUDE_PATTERNS.some((p) => p.test(filePath));
}

async function* walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      yield* walk(fullPath);
    } else if (shouldProcess(fullPath)) {
      yield fullPath;
    }
  }
}

function replaceInTsx(content) {
  // 匹配 fontSize: 13[, }] 或 fontSize: 13 }} 等
  // 也匹配 fontSize: '13px' / "13px"
  return content
    .replace(
      /fontSize\s*:\s*(\d+)\b(?=\s*[,}\]])/g,
      (match, n) => {
        const token = TOKEN_MAP[n];
        return token ? `fontSize: 'var(${token})'` : match;
      },
    )
    .replace(
      /fontSize\s*:\s*['"](\d+)px['"]/g,
      (match, n) => {
        const token = TOKEN_MAP[n];
        return token ? `fontSize: 'var(${token})'` : match;
      },
    );
}

function replaceInCss(content) {
  return content.replace(
    /font-size\s*:\s*(\d+)px\b/g,
    (match, n) => {
      const token = TOKEN_MAP[n];
      return token ? `font-size: var(${token})` : match;
    },
  );
}

async function main() {
  let changedFiles = 0;
  let totalReplacements = 0;

  for await (const filePath of walk(ROOT)) {
    const original = await readFile(filePath, 'utf8');
    const isCss = filePath.endsWith('.css');
    const updated = isCss ? replaceInCss(original) : replaceInTsx(original);

    if (updated !== original) {
      const replacements = (original.match(/fontSize\s*:\s*\d+/g) || []).length
        + (original.match(/font-size\s*:\s*\d+px/g) || []).length;
      totalReplacements += replacements;
      changedFiles += 1;
      await writeFile(filePath, updated, 'utf8');
      console.log(`[${replacements}] ${path.relative(process.cwd(), filePath)}`);
    }
  }

  console.log(`\nDone. ${changedFiles} files changed, ~${totalReplacements} replacements.`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
