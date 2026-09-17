#!/usr/bin/env node
/**
 * 生成桌面 latest.json、固定 legacy 镜像清单及 Android release.json。
 * node scripts/generate-latest-json.js <version> <artifacts-dir> <output-file>
 *   [--notes-file <path>] [--download-base-url <https-base>] [--no-probe]
 *   [--proxies id=https-prefix,...]
 *
 * CDN 只改变分发地址，不改变产物、updater 签名或 Android 校验和签名。
 * 固定镜像文件是已安装客户端的兼容接口，不能根据某次网络探测结果删减。
 */
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const DEFAULT_PROXIES = [
  { id: 'ghfast', prefix: 'https://ghfast.top/' },
  { id: 'ghproxy-net', prefix: 'https://ghproxy.net/' },
  { id: 'ghproxy', prefix: 'https://gh-proxy.com/' },
  { id: 'ghps', prefix: 'https://ghps.cc/' },
];
export const GITHUB_DOWNLOAD_BASE = 'https://github.com/Gczmy/SoloSoul/releases/download';
export const MAX_MANIFEST_BYTES = 512 * 1024;
export const PROBE_BYTES = 1024;
const PROBE_TIMEOUT_MS = 8000;

export function validateVersion(version) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?(?:\+[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$/.test(version)) {
    throw new Error(`无效版本号: ${version}`);
  }
  return version;
}

/** 配置 URL 不允许认证信息、查询参数、片段、歧义路径或反斜杠。 */
export function validateHttpsUrl(value, { allowQuery = false } = {}) {
  if (typeof value !== 'string' || /[\s\\]/u.test(value) || value.includes('#') || (!allowQuery && value.includes('?'))) {
    throw new Error(`必须使用不含认证信息、查询参数或片段的 HTTPS URL: ${value}`);
  }
  const authorityMatch = /^https:\/\/([^/?#]+)/i.exec(value);
  const authority = authorityMatch?.[1];
  if (!authority || authority.includes('@')) throw new Error(`无效 HTTPS URL: ${value}`);
  const parsed = new URL(value);
  if (parsed.protocol !== 'https:' || !parsed.hostname || parsed.username || parsed.password) {
    throw new Error(`无效 HTTPS URL: ${value}`);
  }
  // URL 构造器会消除 ../，必须检查原始路径，避免接受被悄悄改写的源。
  const rawPath = value.slice(authorityMatch[0].length).split('?')[0];
  for (const segment of rawPath.split('/')) {
    const decoded = decodeURIComponent(segment);
    if (decoded === '.' || decoded === '..' || /[/\\\x00-\x1f\x7f]/u.test(decoded)) {
      throw new Error(`URL 路径含不安全片段: ${value}`);
    }
  }
  return parsed;
}

export function normalizeDownloadBase(value) {
  return validateHttpsUrl(value).href.replace(/\/+$/, '');
}

export function encodePathSegment(value) {
  if (!value || value === '.' || value === '..' || /[/\\\x00-\x1f\x7f]/u.test(value)) {
    throw new Error(`不安全的文件名或路径片段: ${value}`);
  }
  return encodeURIComponent(value).replace(/[!'()*]/g, (char) => `%${char.charCodeAt(0).toString(16).toUpperCase()}`);
}

export function assetUrl(base, version, filename) {
  return `${normalizeDownloadBase(base)}/${encodePathSegment(`v${validateVersion(version)}`)}/${encodePathSegment(filename)}`;
}

const readText = (filename) => fs.readFileSync(filename, 'utf8').trim();
const escapeRegex = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const filesIn = (directory) => fs.readdirSync(directory, { withFileTypes: true }).filter((entry) => entry.isFile()).map((entry) => entry.name).sort();

export function buildLatestJson(version, artifactsDir, { downloadBaseUrl, notes, publishedAt = new Date().toISOString() } = {}) {
  validateVersion(version);
  const base = downloadBaseUrl ? normalizeDownloadBase(downloadBaseUrl) : GITHUB_DOWNLOAD_BASE;
  const patterns = [
    ['darwin-aarch64', '(aarch64|arm64)\\.app\\.tar\\.gz'],
    ['darwin-x86_64', 'x64\\.app\\.tar\\.gz'],
    ['windows-x86_64', 'x64-setup\\.exe'],
    ['linux-x86_64', null],
  ];
  const files = filesIn(artifactsDir);
  const platforms = {};
  for (const [platform, suffix] of patterns) {
    const expression = new RegExp(`^SoloSoul_${escapeRegex(version)}${suffix ? `_${suffix}` : '\\.AppImage'}$`);
    const matching = files.filter((name) => expression.test(name));
    if (matching.length > 1) throw new Error(`${platform} 存在多个同版本产物: ${matching.join(', ')}`);
    if (!matching.length) continue;
    const filename = matching[0];
    const signature = readText(path.join(artifactsDir, `${filename}.sig`));
    if (!signature) throw new Error(`签名文件为空: ${filename}.sig`);
    platforms[platform] = { signature, url: assetUrl(base, version, filename) };
  }
  if (!Object.keys(platforms).length) throw new Error(`No installer artifacts found in ${artifactsDir} for version ${version}`);
  return { version, notes: notes ?? `SoloSoul v${version}`, pub_date: publishedAt, platforms };
}

/** 与客户端 GitHubRelease 结构一致；APK 的 SHA-256 和 minisig 必须成组发布。 */
export function buildReleaseJson(version, artifactsDir, { downloadBaseUrl, notes, publishedAt = new Date().toISOString() } = {}) {
  validateVersion(version);
  const base = downloadBaseUrl ? normalizeDownloadBase(downloadBaseUrl) : GITHUB_DOWNLOAD_BASE;
  const files = filesIn(artifactsDir);
  const apks = files.filter((name) => new RegExp(`^SoloSoul_${escapeRegex(version)}_.+\\.apk$`).test(name));
  const assets = [];
  for (const apk of apks) {
    for (const filename of [apk, `${apk}.sha256`, `${apk}.sha256.minisig`]) {
      if (!files.includes(filename)) throw new Error(`Android 发布资产缺失: ${filename}`);
      const size = fs.statSync(path.join(artifactsDir, filename)).size;
      if (!Number.isSafeInteger(size) || size <= 0) throw new Error(`Android 发布资产为空或过大: ${filename}`);
      assets.push({ name: filename, browser_download_url: assetUrl(base, version, filename), size });
    }
  }
  return { tag_name: `v${version}`, body: notes ?? `SoloSoul v${version}`, published_at: publishedAt, assets };
}

export function parseProxyArg(raw) {
  const proxies = DEFAULT_PROXIES.map((proxy) => ({ ...proxy }));
  const seen = new Set();
  for (const pair of raw.split(',')) {
    const split = pair.indexOf('=');
    const id = pair.slice(0, split);
    if (split <= 0 || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(id) || seen.has(id)) {
      throw new Error(`--proxies 格式应为唯一安全 id=https-prefix: ${pair}`);
    }
    seen.add(id);
    const prefix = `${normalizeDownloadBase(pair.slice(split + 1))}/`;
    const existing = proxies.find((proxy) => proxy.id === id);
    if (existing) existing.prefix = prefix;
    else proxies.push({ id, prefix });
  }
  return proxies;
}

/** 有界读取；保留超时直到 body 读取结束，禁止重定向降级到 HTTP。 */
export async function fetchBounded(url, { maxBytes = MAX_MANIFEST_BYTES, headers = {}, timeoutMs = PROBE_TIMEOUT_MS, fetchImpl = fetch, inspectResponse } = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(new Error('请求超时')), timeoutMs);
  let response;
  try {
    let current = validateHttpsUrl(url).href;
    for (let redirects = 0; ; redirects++) {
      response = await fetchImpl(current, { headers, signal: controller.signal, redirect: 'manual' });
      if (![301, 302, 303, 307, 308].includes(response.status)) break;
      await response.body?.cancel();
      if (redirects >= 5) throw new Error('重定向次数超过 5 次');
      const location = response.headers.get('location');
      if (!location) throw new Error('重定向缺少 Location');
      current = validateHttpsUrl(new URL(location, current).href, { allowQuery: true }).href;
    }
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    inspectResponse?.(response);
    const declaredLength = response.headers.get('content-length');
    if (declaredLength !== null && (!/^\d+$/.test(declaredLength) || Number(declaredLength) > maxBytes)) {
      throw new Error(`Content-Length 超过读取上限 ${maxBytes}`);
    }
    if (!response.body) throw new Error('响应正文为空');
    const reader = response.body.getReader();
    const chunks = [];
    let length = 0;
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        length += value.byteLength;
        if (length > maxBytes) throw new Error(`响应正文超过读取上限 ${maxBytes}`);
        chunks.push(Buffer.from(value));
      }
    } finally {
      await reader.cancel().catch(() => {});
    }
    return { response, body: Buffer.concat(chunks, length) };
  } finally {
    controller.abort();
    clearTimeout(timer);
    if (response?.body && !response.body.locked) await response.body.cancel().catch(() => {});
  }
}

/** 检查实际资产的 Range 协议和长度，不读取整个安装包。 */
export async function probeArtifact(url, { expectedSize, expectedPrefix, fetchImpl, timeoutMs } = {}) {
  const bytes = expectedSize ? Math.min(PROBE_BYTES, expectedSize) : PROBE_BYTES;
  let totalSize;
  const { body } = await fetchBounded(url, {
    maxBytes: bytes,
    headers: { Range: `bytes=0-${bytes - 1}`, 'Accept-Encoding': 'identity' },
    fetchImpl, timeoutMs,
    inspectResponse(response) {
      if (response.status !== 206) throw new Error(`资产必须支持 Range/206，实际 HTTP ${response.status}`);
      const match = /^bytes 0-(\d+)\/(\d+)$/.exec(response.headers.get('content-range') ?? '');
      totalSize = match ? Number(match[2]) : 0;
      if (!match || !Number.isSafeInteger(totalSize) || totalSize < bytes || Number(match[1]) !== bytes - 1) {
        throw new Error('Content-Range 与请求不一致');
      }
      if (expectedSize !== undefined && totalSize !== expectedSize) throw new Error(`资产大小不匹配: ${totalSize} != ${expectedSize}`);
      if (/text\/html/i.test(response.headers.get('content-type') ?? '')) throw new Error('资产返回了 HTML 页面');
    },
  });
  if (body.length !== bytes) throw new Error(`Range 正文长度错误: ${body.length} != ${bytes}`);
  if (expectedPrefix && !body.equals(expectedPrefix.subarray(0, bytes))) throw new Error('远端资产开头内容与本地产物不一致');
  return { bytes: body.length, totalSize };
}

export function readArtifactPrefix(filename) {
  const size = fs.statSync(filename).size;
  if (!Number.isSafeInteger(size) || size <= 0) throw new Error(`资产为空或过大: ${filename}`);
  const prefix = Buffer.alloc(Math.min(PROBE_BYTES, size));
  const fd = fs.openSync(filename, 'r');
  try {
    if (fs.readSync(fd, prefix, 0, prefix.length, 0) !== prefix.length) throw new Error(`读取本地产物首段不完整: ${filename}`);
  } finally { fs.closeSync(fd); }
  return { expectedSize: size, expectedPrefix: prefix };
}

export async function generateManifests({ version, artifactsDir, outputFile, notes, downloadBaseUrl, proxies = DEFAULT_PROXIES, noProbe = false, fetchImpl, log = console.log, warn = console.warn }) {
  const publishedAt = new Date().toISOString();
  const options = { notes, downloadBaseUrl, publishedAt };
  const latest = buildLatestJson(version, artifactsDir, options);
  const release = buildReleaseJson(version, artifactsDir, options);
  const outDir = path.dirname(outputFile);
  const outputs = [{ filename: outputFile, manifest: latest }];
  for (const proxy of proxies) {
    const mirror = structuredClone(latest);
    if (!downloadBaseUrl) for (const item of Object.values(mirror.platforms)) item.url = `${proxy.prefix}${item.url}`;
    outputs.push({ filename: path.join(outDir, `latest-mirror-${proxy.id}.json`), manifest: mirror });
  }
  outputs.push({ filename: path.join(outDir, 'release.json'), manifest: release });
  if (new Set(outputs.map(({ filename }) => path.resolve(filename))).size !== outputs.length) {
    throw new Error('output-file 与兼容清单或 release.json 重名');
  }
  // 先完整校验输入，再落盘，避免缺少 APK 签名时留下貌似完整的清单。
  const serialized = outputs.map(({ filename, manifest }) => {
    const json = `${JSON.stringify(manifest, null, 2)}\n`;
    if (Buffer.byteLength(json) > MAX_MANIFEST_BYTES) throw new Error(`清单超过 ${MAX_MANIFEST_BYTES} 字节: ${filename}`);
    return { filename, json };
  });
  fs.mkdirSync(outDir, { recursive: true });
  for (const { filename, json } of serialized) {
    fs.writeFileSync(filename, json);
    log(`Generated ${filename}`);
  }
  if (noProbe) log('--no-probe: 离线生成全部兼容清单，未执行网络探测。');
  else {
    const probeUrls = [...new Set(outputs.flatMap(({ manifest }) => Object.values(manifest.platforms ?? {}).map((item) => item.url)))];
    await Promise.all(probeUrls.map(async (url) => {
      const filename = decodeURIComponent(new URL(url).pathname.split('/').at(-1));
      try {
        const result = await probeArtifact(url, { ...readArtifactPrefix(path.join(artifactsDir, filename)), fetchImpl });
        log(`Range 检查通过: ${url} (${result.totalSize} bytes)`);
      } catch (error) { warn(`Range 检查失败: ${url}: ${error.message}；兼容清单仍保留，发布前需验证真实资产。`); }
    }));
  }
  return outputs;
}

export function parseArgs(args) {
  const options = {};
  const positional = [];
  const flags = { '--notes-file': 'notesFile', '--download-base-url': 'downloadBaseUrl', '--proxies': 'proxyArg' };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--no-probe') options.noProbe = true;
    else if (flags[arg]) {
      if (!args[i + 1] || args[i + 1].startsWith('--')) throw new Error(`${arg} 缺少参数`);
      if (options[flags[arg]] !== undefined) throw new Error(`${arg} 重复指定`);
      options[flags[arg]] = args[++i];
    } else if (arg.startsWith('--')) throw new Error(`未知选项: ${arg}`);
    else positional.push(arg);
  }
  if (positional.length !== 3) throw new Error('Usage: node scripts/generate-latest-json.js <version> <artifacts-dir> <output-file> [--notes-file <path>] [--download-base-url <https-base>] [--no-probe] [--proxies id=prefix,...]');
  const [version, artifactsDir, outputFile] = positional;
  return { version, artifactsDir, outputFile, noProbe: options.noProbe, downloadBaseUrl: options.downloadBaseUrl,
    notes: options.notesFile ? readText(options.notesFile) : undefined,
    proxies: options.proxyArg ? parseProxyArg(options.proxyArg) : DEFAULT_PROXIES };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  Promise.resolve().then(() => generateManifests(parseArgs(process.argv.slice(2)))).catch((error) => { console.error(error.message); process.exitCode = 1; });
}
