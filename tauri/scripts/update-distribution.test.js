import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  DEFAULT_PROXIES, MAX_MANIFEST_BYTES, PROBE_BYTES, assetUrl, buildLatestJson,
  buildReleaseJson, fetchBounded, generateManifests, normalizeDownloadBase,
  parseArgs, parseProxyArg, probeArtifact, validateHttpsUrl, validateVersion,
} from './generate-latest-json.js';
import {
  checkRemoteManifest, inspectLocalManifests, parseVerifyArgs, readManifest,
  validateRelease, verifyDistribution,
} from './verify-update-distribution.js';

const version = '2.13.0';
const cdn = 'https://updates.example.invalid/releases';
const quiet = () => {};

function fixture(t, { android = true, selectedVersion = version } = {}) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'solosoul-updates-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const desktop = `SoloSoul_${selectedVersion}_arm64.app.tar.gz`;
  const windows = `SoloSoul_${selectedVersion}_x64-setup.exe`;
  const apk = `SoloSoul_${selectedVersion}_universal-release.apk`;
  for (const name of [desktop, windows]) {
    fs.writeFileSync(path.join(directory, name), Buffer.alloc(4097, name === desktop ? 3 : 5));
    fs.writeFileSync(path.join(directory, `${name}.sig`), `signature-for-${name}\n`);
  }
  if (android) {
    fs.writeFileSync(path.join(directory, apk), Buffer.alloc(5001, 7));
    fs.writeFileSync(path.join(directory, `${apk}.sha256`), `${'a'.repeat(64)}\n`);
    fs.writeFileSync(path.join(directory, `${apk}.sha256.minisig`), 'existing-minisign-signature\n');
  }
  return { directory, desktop, windows, apk, outputFile: path.join(directory, 'latest.json') };
}

async function generate(t, options = {}) {
  const files = fixture(t);
  await generateManifests({ version, artifactsDir: files.directory, outputFile: files.outputFile, noProbe: true, log: quiet, ...options });
  return files;
}

function rangeResponse(body, total = body.length, overrides = {}) {
  return new Response(body, { status: 206, headers: {
    'content-range': `bytes 0-${body.length - 1}/${total}`, 'content-length': String(body.length),
    'content-type': 'application/octet-stream', ...overrides,
  } });
}

test('资产 URL 按片段编码版本、空格、中文、井号、问号和百分号', () => {
  assert.equal(assetUrl(`${cdn}/`, '2.13.0+build.1', '安装包 +?#%.apk'), `${cdn}/v2.13.0%2Bbuild.1/%E5%AE%89%E8%A3%85%E5%8C%85%20%2B%3F%23%25.apk`);
  assert.equal(normalizeDownloadBase('https://HTTPS/releases///'), 'https://https/releases');
  for (const filename of ['..', '../bad.apk', 'bad\\file', 'bad\0file']) assert.throws(() => assetUrl(cdn, version, filename));
});

test('拒绝非 HTTPS、userinfo、查询、片段、控制字符和歧义路径', () => {
  for (const url of ['http://host/releases', 'file:///tmp/a', '//host/a', 'https://user:pass@host/a', 'https://@host/a', 'https://host/a?', 'https://host/a#', 'https://host/a?q=1', 'https://host/a#f', ' https://host/a', 'https://host/../a', 'https://host/%2e%2e/a', 'https://host/%2fetc', 'https://host/%5cfoo', 'https://host/%00a', 'https://host/\\a', 'https://host/%broken']) {
    assert.throws(() => validateHttpsUrl(url), url);
  }
  assert.equal(normalizeDownloadBase('https://host:8443/soul/releases/'), 'https://host:8443/soul/releases');
  for (const value of ['../2.13.0', 'v2.13.0', '2.13.0/x', '2.13.0?x', '2x13x0', '02.13.0']) assert.throws(() => validateVersion(value));
});

test('--no-probe 稳定生成全部 legacy 文件，不发生网络请求，签名和元数据一致', async (t) => {
  const files = await generate(t, { fetchImpl: () => assert.fail('不应联网') });
  const local = inspectLocalManifests(files.outputFile);
  assert.equal(local.manifestCount, 6);
  const latest = readManifest(files.outputFile);
  for (const { id, prefix } of DEFAULT_PROXIES) {
    const mirror = readManifest(path.join(files.directory, `latest-mirror-${id}.json`));
    assert.equal(mirror.pub_date, latest.pub_date);
    assert.equal(mirror.platforms['darwin-aarch64'].signature, latest.platforms['darwin-aarch64'].signature);
    assert.equal(mirror.platforms['darwin-aarch64'].url, `${prefix}${latest.platforms['darwin-aarch64'].url}`);
  }
});

test('CDN 覆盖主清单、全部 legacy 清单和 Android URL，现有签名不变', async (t) => {
  const files = await generate(t, { downloadBaseUrl: `${cdn}/`, notes: '发布正文\n[MANDATORY]' });
  const latest = readManifest(files.outputFile);
  for (const { id } of DEFAULT_PROXIES) {
    assert.deepEqual(readManifest(path.join(files.directory, `latest-mirror-${id}.json`)), latest);
  }
  const artifact = latest.platforms['darwin-aarch64'];
  assert.equal(artifact.url, `${cdn}/v${version}/${files.desktop}`);
  assert.equal(artifact.signature, fs.readFileSync(path.join(files.directory, `${files.desktop}.sig`), 'utf8').trim());
  const release = readManifest(path.join(files.directory, 'release.json'));
  assert.equal(release.tag_name, `v${version}`);
  assert.equal(release.body, latest.notes);
  assert.equal(release.published_at, latest.pub_date);
  assert.equal(release.assets.length, 3);
  for (const asset of release.assets) {
    assert.equal(asset.browser_download_url, `${cdn}/v${version}/${asset.name}`);
    assert.equal(asset.size, fs.statSync(path.join(files.directory, asset.name)).size);
  }
  assert.equal(fs.readFileSync(path.join(files.directory, `${files.apk}.sha256.minisig`), 'utf8'), 'existing-minisign-signature\n');
});

test('网络探测失败只输出诊断，不删减 legacy；探测真实包而非 release 页面', async (t) => {
  const files = fixture(t);
  const warnings = [];
  const urls = [];
  await generateManifests({ version, artifactsDir: files.directory, outputFile: files.outputFile, log: quiet, warn: (message) => warnings.push(message), fetchImpl: async (url, options) => {
    urls.push(url);
    assert.equal(options.headers.Range, `bytes=0-${PROBE_BYTES - 1}`);
    return new Response('HTML error', { status: 200 });
  } });
  assert.equal(warnings.length, 10);
  assert(urls.every((url) => /releases\/download\/v2\.13\.0\/SoloSoul_/.test(url)));
  assert.equal(inspectLocalManifests(files.outputFile).manifestCount, 6);
});

test('APK 缺少校验和或签名时拒绝生成，不留下半套清单；桌面独立发布仍有效', async (t) => {
  const files = fixture(t);
  fs.unlinkSync(path.join(files.directory, `${files.apk}.sha256.minisig`));
  await assert.rejects(generateManifests({ version, artifactsDir: files.directory, outputFile: files.outputFile, noProbe: true, log: quiet }), /Android 发布资产缺失/);
  assert(!fs.existsSync(files.outputFile));
  const desktopOnly = fixture(t, { android: false });
  assert.deepEqual(buildReleaseJson(version, desktopOnly.directory).assets, []);
});

test('版本中 + 和 prerelease 不被解释为正则；不能匹配旧版本/多个同平台包', (t) => {
  const files = fixture(t, { selectedVersion: '2.13.0-rc.1+build.2' });
  const latest = buildLatestJson('2.13.0-rc.1+build.2', files.directory);
  assert.equal(Object.keys(latest.platforms).length, 2);
  assert.match(latest.platforms['darwin-aarch64'].url, /v2\.13\.0-rc\.1%2Bbuild\.2/);
  assert.throws(() => buildLatestJson(version, files.directory), /No installer/);
  fs.writeFileSync(path.join(files.directory, files.desktop.replace('arm64', 'aarch64')), 'extra');
  assert.throws(() => buildLatestJson('2.13.0-rc.1+build.2', files.directory), /多个同版本产物/);
});

test('自定义代理保留固定 legacy id，并拒绝文件路径注入与重复 id', () => {
  const proxies = parseProxyArg('custom=https://proxy.example.invalid/,ghfast=https://alternate.example.invalid/');
  assert.equal(proxies.length, 5);
  assert.equal(proxies[0].id, 'ghfast');
  assert.equal(proxies[0].prefix, 'https://alternate.example.invalid/');
  for (const raw of ['../bad=https://host/', '=https://host/', 'id=http://host/', 'id=https://host/,id=https://other/']) assert.throws(() => parseProxyArg(raw));
  assert.throws(() => parseArgs(['2.13.0', '/tmp', '/tmp/latest.json', '--download-base-url']), /缺少参数/);
  assert.throws(() => parseArgs(['2.13.0', '/tmp', '/tmp/latest.json', '--typo']), /未知选项/);
});

test('Range 检查接受精确 206、总大小及本地首段，拒绝 200/伪 Range/HTML/截断/错误内容', async () => {
  const bytes = Buffer.alloc(PROBE_BYTES, 1);
  const valid = { expectedSize: 5000, expectedPrefix: bytes, fetchImpl: async () => rangeResponse(bytes, 5000) };
  assert.deepEqual(await probeArtifact(`${cdn}/file`, valid), { bytes: PROBE_BYTES, totalSize: 5000 });
  const invalidResponses = [
    () => new Response(bytes, { status: 200 }),
    () => new Response(bytes, { status: 206 }),
    () => rangeResponse(bytes, 5001),
    () => rangeResponse(bytes, 5000, { 'content-range': 'bytes 1-1024/5000' }),
    () => rangeResponse(bytes, 5000, { 'content-type': 'text/html' }),
    () => rangeResponse(bytes.subarray(0, 12), 5000, { 'content-range': 'bytes 0-1023/5000' }),
    () => rangeResponse(Buffer.alloc(PROBE_BYTES, 2), 5000),
    () => rangeResponse(Buffer.alloc(PROBE_BYTES + 1), 5000, { 'content-range': 'bytes 0-1023/5000' }),
  ];
  for (const response of invalidResponses) await assert.rejects(probeArtifact(`${cdn}/file`, { ...valid, fetchImpl: async () => response() }));
  const checksum = Buffer.from('abcd');
  assert.deepEqual(await probeArtifact(`${cdn}/checksum`, { expectedSize: 4, expectedPrefix: checksum, fetchImpl: async (_url, options) => {
    assert.equal(options.headers.Range, 'bytes=0-3');
    return rangeResponse(checksum);
  } }), { bytes: 4, totalSize: 4 });
});

test('有界读取拒绝未知长度超限流、过大 Content-Length，限制重定向并拒绝 HTTP 降级', async () => {
  let cancelled = false;
  const body = new ReadableStream({ pull(controller) { controller.enqueue(new Uint8Array(8)); }, cancel() { cancelled = true; } });
  await assert.rejects(fetchBounded(`${cdn}/file`, { maxBytes: 12, fetchImpl: async () => new Response(body) }), /读取上限/);
  assert(cancelled);
  await assert.rejects(fetchBounded(`${cdn}/file`, { maxBytes: 12, fetchImpl: async () => new Response('x', { headers: { 'content-length': '5000' } }) }), /Content-Length/);
  await assert.rejects(fetchBounded(`${cdn}/file`, { maxBytes: 12, fetchImpl: async () => new Response(null, { status: 302, headers: { location: 'http://host/file' } }) }), /HTTPS/);
  await assert.rejects(fetchBounded(`${cdn}/file`, { maxBytes: 12, fetchImpl: async () => new Response(null, { status: 302, headers: { location: '/again' } }) }), /重定向次数/);
  let calls = 0;
  const redirected = await fetchBounded(`${cdn}/file`, { maxBytes: 12, fetchImpl: async (url) => {
    if (calls++ === 0) return new Response(null, { status: 302, headers: { location: 'https://signed.example.invalid/file?signature=value' } });
    assert.equal(url, 'https://signed.example.invalid/file?signature=value');
    return new Response('ok');
  } });
  assert.equal(redirected.body.toString(), 'ok');
});

test('超时覆盖正文读取，而不只是收到响应头之前', async () => {
  await assert.rejects(fetchBounded(`${cdn}/file`, { maxBytes: 12, timeoutMs: 20, fetchImpl: async (_url, { signal }) => new Response(new ReadableStream({
    start(controller) { signal.addEventListener('abort', () => controller.error(signal.reason), { once: true }); },
  })) }), /超时/);
});

test('远端 metadata 必须与本地候选一致，HTML/过大内容被拒绝', async () => {
  const expected = { version, notes: 'text' };
  await checkRemoteManifest(`${cdn}/latest.json`, expected, { fetchImpl: async () => Response.json(expected) });
  await assert.rejects(checkRemoteManifest(`${cdn}/latest.json`, expected, { fetchImpl: async () => Response.json({ version: '2.12.1' }) }), /不一致/);
  await assert.rejects(checkRemoteManifest(`${cdn}/latest.json`, expected, { fetchImpl: async () => new Response('<html>', { headers: { 'content-type': 'text/html' } }) }), /HTML/);
  await assert.rejects(checkRemoteManifest(`${cdn}/latest.json`, expected, { fetchImpl: async () => new Response('a'.repeat(MAX_MANIFEST_BYTES + 1)) }), /读取上限/);
});

test('完整只读验收脚本检查远端两份 Android metadata、主清单和所有资产', async (t) => {
  const files = await generate(t, { downloadBaseUrl: cdn });
  const local = inspectLocalManifests(files.outputFile);
  const seen = [];
  await verifyDistribution({ latestFile: files.outputFile, manifestUrls: [`${cdn}/latest/latest.json`], releaseUrls: [`${cdn}/latest/release.json`, `${cdn}/v${version}/release.json`], log: quiet, fetchImpl: async (url, options) => {
    seen.push(url);
    if (url.endsWith('/latest.json')) return Response.json(local.latest);
    if (url.endsWith('/release.json')) return Response.json(local.release);
    assert.equal(options.headers['Accept-Encoding'], 'identity');
    const name = decodeURIComponent(new URL(url).pathname.split('/').at(-1));
    const file = fs.readFileSync(path.join(files.directory, name));
    return rangeResponse(file.subarray(0, Math.min(PROBE_BYTES, file.length)), file.length);
  } });
  assert.equal(seen.length, 8);
  await verifyDistribution({ latestFile: files.outputFile, offline: true, log: quiet, fetchImpl: () => assert.fail('离线验收不应联网') });
  await assert.rejects(verifyDistribution({ latestFile: files.outputFile, log: quiet, fetchImpl: async () => new Response('bad', { status: 502 }) }), /5 项分发验收失败/);
});

test('本地验收拒绝缺失 legacy、签名不一致、Android asset 大小及命名组合错误', async (t) => {
  const files = await generate(t, { downloadBaseUrl: cdn });
  const filename = path.join(files.directory, 'latest-mirror-ghfast.json');
  const original = fs.readFileSync(filename);
  fs.unlinkSync(filename);
  assert.throws(() => inspectLocalManifests(files.outputFile), /ENOENT/);
  const modified = JSON.parse(original);
  modified.platforms['darwin-aarch64'].signature = 'changed';
  fs.writeFileSync(filename, JSON.stringify(modified));
  assert.throws(() => inspectLocalManifests(files.outputFile), /签名/);
  fs.writeFileSync(filename, original);
  const latest = readManifest(files.outputFile);
  const release = readManifest(path.join(files.directory, 'release.json'));
  assert.throws(() => validateRelease({ ...release, assets: release.assets.slice(0, 1) }, latest), /完整/);
  release.assets[0].size++;
  fs.writeFileSync(path.join(files.directory, 'release.json'), JSON.stringify(release));
  assert.throws(() => inspectLocalManifests(files.outputFile), /size 与本地/);
});

test('CLI 离线端到端生成/验收且参数错误退出非零', (t) => {
  const files = fixture(t);
  const directory = path.dirname(fileURLToPath(import.meta.url));
  const generator = path.join(directory, 'generate-latest-json.js');
  const verifier = path.join(directory, 'verify-update-distribution.js');
  const notes = path.join(files.directory, 'release-notes.md');
  fs.writeFileSync(notes, '离线 CLI 正文');
  execFileSync(process.execPath, [generator, version, files.directory, files.outputFile, '--no-probe', '--notes-file', notes, '--download-base-url', cdn]);
  const output = execFileSync(process.execPath, [verifier, files.outputFile, '--offline'], { encoding: 'utf8' });
  assert.match(output, /本地检查通过/);
  assert.equal(readManifest(files.outputFile).notes, '离线 CLI 正文');
  assert.equal(spawnSync(process.execPath, [generator, version, files.directory, files.outputFile, '--download-base-url']).status, 1);
  assert.equal(spawnSync(process.execPath, [verifier, files.outputFile, '--unexpected']).status, 1);
  assert.throws(() => parseVerifyArgs([files.outputFile, '--manifest-url', 'http://bad/']), /HTTPS/);
});
