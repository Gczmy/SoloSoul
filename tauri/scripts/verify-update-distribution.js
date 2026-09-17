#!/usr/bin/env node
/**
 * 只读验收：校验本地清单、兼容清单、签名一致性及远端真实资产 Range。
 * 安装包仅读最多 1024 字节；本脚本不替代完整 SHA-256 / minisign 验证。
 * node scripts/verify-update-distribution.js <latest-file> [--artifacts-dir <dir>]
 *   [--manifest-url <url>]... [--release-url <url>]... [--offline]
 */
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import {
  DEFAULT_PROXIES, MAX_MANIFEST_BYTES, encodePathSegment, fetchBounded,
  probeArtifact, readArtifactPrefix, validateHttpsUrl, validateVersion,
} from './generate-latest-json.js';

const assert = (condition, message) => { if (!condition) throw new Error(message); };

export function readManifest(filename) {
  assert(fs.statSync(filename).size <= MAX_MANIFEST_BYTES, `清单超过 ${MAX_MANIFEST_BYTES} 字节: ${filename}`);
  return JSON.parse(fs.readFileSync(filename, 'utf8'));
}

function assetFilename(url) {
  const parsed = validateHttpsUrl(url);
  const filename = decodeURIComponent(parsed.pathname.split('/').at(-1));
  encodePathSegment(filename);
  return filename;
}

export function validateLatest(manifest) {
  assert(manifest && typeof manifest === 'object', 'latest.json 必须是对象');
  validateVersion(manifest.version);
  assert(typeof manifest.notes === 'string', 'latest.json 缺少 notes');
  assert(typeof manifest.pub_date === 'string' && Number.isFinite(Date.parse(manifest.pub_date)), 'latest.json pub_date 无效');
  assert(manifest.platforms && !Array.isArray(manifest.platforms) && typeof manifest.platforms === 'object' && Object.keys(manifest.platforms).length > 0, 'latest.json 缺少 platforms');
  for (const [platform, asset] of Object.entries(manifest.platforms)) {
    assert(asset && typeof asset.signature === 'string' && asset.signature.trim(), `${platform} 缺少 updater 签名`);
    const filename = assetFilename(asset.url);
    assert(filename.startsWith(`SoloSoul_${manifest.version}`), `${platform} 资产版本不匹配`);
  }
}

export function validateRelease(release, latest) {
  assert(release && release.tag_name === `v${latest.version}`, 'release.json 版本与 latest.json 不一致');
  assert(release.body === latest.notes && release.published_at === latest.pub_date, 'release.json 正文或发布时间与 latest.json 不一致');
  assert(Array.isArray(release.assets), 'release.json 缺少 assets');
  const assets = new Map();
  for (const asset of release.assets) {
    assert(asset && typeof asset.name === 'string' && !assets.has(asset.name), 'release.json 资产名无效或重复');
    assert(assetFilename(asset.browser_download_url) === asset.name, `资产 URL 与名称不一致: ${asset.name}`);
    assert(asset.name.startsWith(`SoloSoul_${latest.version}_`) && /\.apk(?:\.sha256(?:\.minisig)?)?$/.test(asset.name), `未知 Android 资产: ${asset.name}`);
    assert(Number.isSafeInteger(asset.size) && asset.size > 0, `资产 size 无效: ${asset.name}`);
    assets.set(asset.name, asset);
  }
  for (const name of assets.keys()) {
    const apk = name.replace(/\.sha256(?:\.minisig)?$/, '');
    for (const filename of [apk, `${apk}.sha256`, `${apk}.sha256.minisig`]) {
      assert(assets.has(filename), `Android 资产缺少完整 APK/校验和/签名组合: ${filename}`);
    }
  }
}

export function inspectLocalManifests(latestFile, artifactsDir = path.dirname(latestFile)) {
  const latest = readManifest(latestFile);
  validateLatest(latest);
  const directory = path.dirname(latestFile);
  const mirrors = DEFAULT_PROXIES.map(({ id }) => `latest-mirror-${id}.json`);
  // 默认兼容文件必须全部存在，额外镜像也要校验。
  const extraMirrors = fs.readdirSync(directory).filter((name) => /^latest-mirror-[a-z0-9-]+\.json$/.test(name));
  const desktopManifests = [latest];
  for (const name of new Set([...mirrors, ...extraMirrors])) {
    const mirror = readManifest(path.join(directory, name));
    validateLatest(mirror);
    assert(mirror.version === latest.version && mirror.notes === latest.notes && mirror.pub_date === latest.pub_date, `${name} 元数据不一致`);
    assert(isDeepStrictEqual(Object.keys(mirror.platforms).sort(), Object.keys(latest.platforms).sort()), `${name} 平台集合不一致`);
    for (const [platform, asset] of Object.entries(latest.platforms)) {
      assert(mirror.platforms[platform].signature === asset.signature, `${name} ${platform} 签名与主清单不一致`);
      assert(assetFilename(mirror.platforms[platform].url) === assetFilename(asset.url), `${name} ${platform} 安装包不一致`);
    }
    desktopManifests.push(mirror);
  }
  const release = readManifest(path.join(directory, 'release.json'));
  validateRelease(release, latest);
  const targets = new Map();
  for (const manifest of desktopManifests) {
    for (const asset of Object.values(manifest.platforms)) {
      const filename = path.join(artifactsDir, assetFilename(asset.url));
      const signature = fs.readFileSync(`${filename}.sig`, 'utf8').trim();
      assert(signature === asset.signature, `${filename}.sig 与清单签名不一致`);
      targets.set(asset.url, { url: asset.url, ...readArtifactPrefix(filename) });
    }
  }
  for (const asset of release.assets) {
    const local = readArtifactPrefix(path.join(artifactsDir, asset.name));
    assert(local.expectedSize === asset.size, `${asset.name} size 与本地文件不一致`);
    targets.set(asset.browser_download_url, { url: asset.browser_download_url, ...local });
  }
  return { latest, release, targets: [...targets.values()], manifestCount: desktopManifests.length + 1 };
}

export async function checkRemoteManifest(url, expected, { fetchImpl } = {}) {
  const { body } = await fetchBounded(url, {
    maxBytes: MAX_MANIFEST_BYTES, fetchImpl,
    inspectResponse(response) {
      assert(response.status === 200, `元数据应返回 HTTP 200，实际 ${response.status}`);
      assert(!/text\/html/i.test(response.headers.get('content-type') ?? ''), '元数据返回了 HTML 页面');
    },
  });
  const actual = JSON.parse(body.toString('utf8'));
  assert(isDeepStrictEqual(actual, expected), `远端元数据与本地候选清单不一致: ${url}`);
}

export async function verifyDistribution({ latestFile, artifactsDir, manifestUrls = [], releaseUrls = [], offline = false, fetchImpl, log = console.log }) {
  const local = inspectLocalManifests(latestFile, artifactsDir);
  log(`本地检查通过: ${local.manifestCount} 份清单，${local.targets.length} 个资产 URL；签名与本地 .sig 一致。`);
  if (offline) {
    assert(!manifestUrls.length && !releaseUrls.length, '--offline 不能同时指定远端清单 URL');
    log('离线检查完成；尚未验证远端资产或完整二进制签名。');
    return local;
  }
  const checks = [
    ...manifestUrls.map((url) => ({ url, run: () => checkRemoteManifest(url, local.latest, { fetchImpl }) })),
    ...releaseUrls.map((url) => ({ url, run: () => checkRemoteManifest(url, local.release, { fetchImpl }) })),
    ...local.targets.map((target) => ({ url: target.url, run: () => probeArtifact(target.url, { ...target, fetchImpl }) })),
  ];
  // 限制并发，避免所有镜像和资产同时建连。
  const failures = [];
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(4, checks.length) }, async () => {
    while (next < checks.length) {
      const { url, run } = checks[next++];
      try { await run(); log(`PASS ${url}`); }
      catch (error) { failures.push(`${url}: ${error.message}`); log(`FAIL ${url}: ${error.message}`); }
    }
  }));
  assert(!failures.length, `${failures.length} 项分发验收失败:\n${failures.join('\n')}`);
  log('分发验收通过。安装包仅读取首段；完整 SHA-256 / minisign 仍由签名自检和客户端验证。');
  return local;
}

export function parseVerifyArgs(args) {
  const options = { manifestUrls: [], releaseUrls: [] };
  const positional = [];
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--offline') options.offline = true;
    else if (['--artifacts-dir', '--manifest-url', '--release-url'].includes(arg)) {
      const value = args[++i];
      assert(value && !value.startsWith('--'), `${arg} 缺少参数`);
      if (arg === '--artifacts-dir') options.artifactsDir = value;
      else {
        validateHttpsUrl(value);
        options[arg === '--manifest-url' ? 'manifestUrls' : 'releaseUrls'].push(value);
      }
    } else if (arg.startsWith('--')) throw new Error(`未知选项: ${arg}`);
    else positional.push(arg);
  }
  assert(positional.length === 1, 'Usage: node scripts/verify-update-distribution.js <latest-file> [--artifacts-dir <dir>] [--manifest-url <url>]... [--release-url <url>]... [--offline]');
  options.latestFile = positional[0];
  return options;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  Promise.resolve().then(() => verifyDistribution(parseVerifyArgs(process.argv.slice(2)))).catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
