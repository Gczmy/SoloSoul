/**
 * 扫描 src 下所有 t('ns:key') / t('key') 用法，与 locales 的 8 个命名空间比对，
 * 报告两份语言（zh-CN / en-US）中缺失的键。
 *
 * 用法：node scripts/check-missing-i18n.mjs
 * 说明：仅静态扫描显式 `t('namespace:key')` 字面量（含嵌套 key 如 editor:field_types.dynamic_group，
 *       按 JSON 路径逐段判定）；不含 defaultValue 的调用若键缺失即为隐患（会显示裸键名）。
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, extname } from 'node:path';

const ROOT = new URL('..', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1');
const SRC = join(ROOT, 'src');
const LOCALES = join(ROOT, 'src/locales');

// 1. 收集源码里的 t('...') 字面量
function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) {
      if (name === 'locales' || name === 'test') continue;
      walk(p, out);
    } else if (extname(p) === '.ts' || extname(p) === '.tsx') {
      out.push(p);
    }
  }
  return out;
}

const files = walk(SRC);
// ns -> Set(keyPath)；判定时若键在任一已加载命名空间存在即算解析成功（i18next 依次回退）
const used = new Map();
// 文件 -> 命名空间列表（仅用于统计，无前缀键按并集判定）
const fileNs = new Map();

// 仅匹配纯字符串字面量：t('ns:key') 后必须跟 ) 或 ,（排除 t('settings:' + x) 拼接）
const T_CALL = /\bt\(\s*['"]([^'"]+)['"]\s*(?=[,)"])/g;
const T_INTERP = /t\?\?\(['"]([^'"]+)['"]\s*(?=[,)"])/g;
// useTranslation('plugin') / useTranslation(['plugin','common','navigation'])
const UT_STR_RE = /useTranslation\(\s*['"]([^'"]+)['"]\s*\)/;
const UT_ARR_RE = /useTranslation\(\s*\[([^\]]*)\]\s*\)/;

for (const file of files) {
  const text = readFileSync(file, 'utf8');
  const arr = text.match(UT_ARR_RE);
  const defNs = arr
    ? [...arr[1].matchAll(/['"]([^'"]+)['"]/g)].map((m) => m[1])
    : (text.match(UT_STR_RE)?.[1] ?? 'common');
  const nsList = Array.isArray(defNs) ? defNs : [defNs];
  fileNs.set(file, nsList);
  const hits = [];
  for (const re of [T_CALL, T_INTERP]) {
    let mm;
    while ((mm = re.exec(text)) !== null) {
      const key = mm[1];
      if (key.includes(' ')) continue; // 非键（如模板字符串）
      hits.push(key);
    }
  }
  for (const key of hits) {
    const colon = key.indexOf(':');
    // 显式前缀：只归入该命名空间；无前缀：归入文件加载的全部命名空间（回退可达）
    const nsList2 = colon > 0 ? [key.slice(0, colon)] : nsList;
    const kp = colon > 0 ? key.slice(colon + 1) : key;
    for (const ns of nsList2) {
      if (!used.has(ns)) used.set(ns, new Set());
      used.get(ns).add(kp);
    }
  }
}

// 2. 加载 locale JSON，按路径解析键
function load(lang) {
  const base = join(LOCALES, lang);
  const out = new Map(); // ns -> Set of existing dotted paths
  for (const f of readdirSync(base)) {
    const ns = f.replace(/\.json$/, '');
    const data = JSON.parse(readFileSync(join(base, f), 'utf8'));
    const paths = new Set();
    (function walkPaths(obj, prefix) {
      for (const [k, v] of Object.entries(obj)) {
        const p = prefix ? `${prefix}.${k}` : k;
        if (v && typeof v === 'object' && !Array.isArray(v)) walkPaths(v, p);
        else paths.add(p);
      }
    })(data, '');
    out.set(ns, paths);
  }
  return out;
}

const zh = load('zh-CN');
const en = load('en-US');
const validNs = new Set([...zh.keys()]);

let missingZh = 0;
let missingEn = 0;

const existsIn = (map, ns, kp) => {
  const parts = kp.split('.');
  const full = parts.join('.');
  if (map.get(ns)?.has(full)) return true;
  for (let i = 1; i < parts.length; i++) {
    if (map.get(ns)?.has(parts.slice(0, i).join('.'))) return true;
  }
  return false;
};

for (const [ns, keys] of used) {
  if (!validNs.has(ns)) {
    console.log(`⚠️  未知命名空间 ${ns}（未加载对应 locale 文件）`);
    continue;
  }
  for (const kp of keys) {
    // i18next 依次回退：任一已加载命名空间存在即解析成功
    const inZh = [...zh.keys()].some((n) => existsIn(zh, n, kp));
    const inEn = [...en.keys()].some((n) => existsIn(en, n, kp));
    if (!inZh) {
      missingZh++;
      console.log(`❌ zh-CN 缺失: ${ns}:${kp}`);
    }
    if (!inEn) {
      missingEn++;
      console.log(`❌ en-US 缺失: ${ns}:${kp}`);
    }
  }
}

console.log(`\n扫描 ${files.length} 个源文件，${used.size} 个命名空间`);
console.log(`zh-CN 缺失 ${missingZh} 个键，en-US 缺失 ${missingEn} 个键`);
