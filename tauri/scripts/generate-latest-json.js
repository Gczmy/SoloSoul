#!/usr/bin/env node
/**
 * 根据构建产物生成 Tauri updater 所需的 latest.json（及国内加速镜像清单）。
 *
 * 用法：
 *   node scripts/generate-latest-json.js <version> <artifacts-dir> <output-file> [--notes-file <path>] [--no-probe] [--proxies id=prefix,id=prefix]
 *
 * 参数：
 *   - version: 应用版本号（例如 2.1.0）
 *   - artifacts-dir: 存放 .dmg/.exe/.AppImage 及对应 .sig 的目录
 *   - output-file: 输出的 latest.json 路径
 *   - --notes-file <path>: release notes 的 markdown 文件路径（写入 latest.json 的 notes 字段，
 *                          桌面端更新横幅「查看更新内容」据此渲染完整正文；缺省时回退为
 *                          占位符 `SoloSoul v<version>`）。发版前先写好该文件再生成清单。
 *   - --no-probe: 跳过代理探测（只生成直连 latest.json，不发网络请求）
 *   - --proxies: 覆盖默认代理列表，格式 `id1=https://prefix1/,id2=https://prefix2/`
 *
 * 产物（发版时探测选最快）：
 *   - latest.json                         直连 GitHub URL 清单（全球用户默认通道）
 *   - latest-mirror-<id>.json             每个探测存活且按延迟排序的代理各一份，
 *                                          清单内平台 URL 均带该代理前缀（元数据与下载
 *                                          走同一通道，避免「元数据通了、下载 URL 却指向
 *                                          死代理」的不一致）；全部探测失败时仅生成 latest.json。
 *
 * 安全说明：镜像清单与直连清单的签名一致（签名针对安装包二进制而非清单本身），
 * Tauri updater 无论从哪个 endpoint 下载都会用 pubkey 严格验签，指向代理无供应链风险。
 *
 * 隐私/可用性披露（T004）：gh-proxy 类代理是 TLS 终止代理——连接在代理方解密后
 * 转发，用户 IP、使用 SoloSoul 的事实、目标版本号、GitHub 响应内容都会暴露给第三方
 * 代理服务商（直连优先，直连可达时不会走代理，直连受限时固有权衡无法消除）；代理
 * 可返回陈旧/篡改的 Release JSON 或重放旧版 mirror JSON 软性压制升级（内容完整性
 * 不受影响——安装包仍强制验签，属可用性面；updater 只升不降，无降级风险）。
 */

import fs from 'fs';
import path from 'path';

// 国内 GitHub 加速代理候选（与 src-tauri/src/commands/update.rs 的 PROXY_PREFIXES 同源维护）。
// 探测时按存活且延迟排序；`--proxies` 可覆盖。失效条目直接替换即可。
const DEFAULT_PROXIES = [
  { id: 'ghfast', prefix: 'https://ghfast.top/' },
  { id: 'ghproxy-net', prefix: 'https://ghproxy.net/' },
  { id: 'ghproxy', prefix: 'https://gh-proxy.com/' },
  { id: 'ghps', prefix: 'https://ghps.cc/' },
];

/** 单个代理探测超时（毫秒）。 */
const PROBE_TIMEOUT_MS = 8000;
/**
 * 探测目标：release 下载路径（与真实更新下载路径一致）。
 *
 * 注意：不要用仓库主页（`/Gczmy/SoloSoul`）——ghproxy 类服务对主页返回 403
 * 是正常的访问控制（只放行 `/releases/...` 路径），会导致存活代理被误判为不可用。
 */
const PROBE_TARGET = 'https://github.com/Gczmy/SoloSoul/releases/latest';

function readFile(filePath) {
  return fs.readFileSync(filePath, 'utf-8').trim();
}

function findFiles(dir, pattern) {
  return fs.readdirSync(dir).filter((name) => pattern.test(name));
}

function resolveSignature(installerPath) {
  const sigPath = `${installerPath}.sig`;
  if (!fs.existsSync(sigPath)) {
    throw new Error(`Missing signature file for ${installerPath}: ${sigPath}`);
  }
  return readFile(sigPath);
}

/**
 * 构建最新清单。urlPrefix 为空 → 直连 URL；非空 → 每个 URL 加代理前缀。
 * notes 缺省时为占位符 `SoloSoul v<version>`（与旧行为一致）。
 */
function buildLatestJson(version, artifactsDir, urlPrefix = '', notes) {
  const wrap = (url) => `${urlPrefix}${url}`;
  const platforms = {};
  // 文件名中包含版本号，避免目录中旧版本产物干扰
  const versionPattern = version.replace(/\./g, '\\.');

  // macOS .app.tar.gz (Apple Silicon) — Tauri updater on macOS expects a gzipped tar archive
  const macArmAppTarGz = findFiles(artifactsDir, new RegExp(`SoloSoul_${versionPattern}_(aarch64|arm64)\\.app\\.tar\\.gz$`))[0];
  if (macArmAppTarGz) {
    platforms['darwin-aarch64'] = {
      signature: resolveSignature(path.join(artifactsDir, macArmAppTarGz)),
      url: wrap(`https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${macArmAppTarGz}`),
    };
  }

  // macOS .app.tar.gz (Intel)
  const macIntelAppTarGz = findFiles(artifactsDir, new RegExp(`SoloSoul_${versionPattern}_x64\\.app\\.tar\\.gz$`))[0];
  if (macIntelAppTarGz) {
    platforms['darwin-x86_64'] = {
      signature: resolveSignature(path.join(artifactsDir, macIntelAppTarGz)),
      url: wrap(`https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${macIntelAppTarGz}`),
    };
  }

  // Windows NSIS installer
  const windowsExe = findFiles(artifactsDir, new RegExp(`SoloSoul_${versionPattern}_x64-setup\\.exe$`))[0];
  if (windowsExe) {
    platforms['windows-x86_64'] = {
      signature: resolveSignature(path.join(artifactsDir, windowsExe)),
      url: wrap(`https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${windowsExe}`),
    };
  }

  // Linux AppImage
  const linuxAppImage = findFiles(artifactsDir, new RegExp(`SoloSoul_${versionPattern}\\.AppImage$`))[0];
  if (linuxAppImage) {
    platforms['linux-x86_64'] = {
      signature: resolveSignature(path.join(artifactsDir, linuxAppImage)),
      url: wrap(`https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${linuxAppImage}`),
    };
  }

  if (Object.keys(platforms).length === 0) {
    throw new Error(`No installer artifacts found in ${artifactsDir} for version ${version}`);
  }

  return {
    version,
    notes: notes ?? `SoloSoul v${version}`,
    pub_date: new Date().toISOString(),
    platforms,
  };
}

/**
 * 探测单个代理：GET 代理前缀 + 仓库主页，仅读取响应头即取消 body。
 * 返回 { alive, latencyMs }。
 */
async function probeProxy(proxy) {
  const url = `${proxy.prefix}${PROBE_TARGET}`;
  const started = Date.now();
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);
    const resp = await fetch(url, { method: 'GET', signal: controller.signal });
    clearTimeout(timer);
    // 读取完头部即放弃 body（避免下载整个页面）
    await resp.body?.cancel().catch(() => {});
    return { proxy, alive: resp.ok, latencyMs: Date.now() - started, status: resp.status };
  } catch (err) {
    return { proxy, alive: false, latencyMs: Date.now() - started, error: String(err) };
  }
}

/**
 * 探测所有代理，返回按延迟升序的存活代理列表（探测失败的不参与生成）。
 */
async function probeProxies(proxies) {
  const results = await Promise.all(proxies.map(probeProxy));
  for (const r of results) {
    if (r.alive) {
      console.log(`  ✅ ${r.proxy.id} 存活（${r.latencyMs}ms, HTTP ${r.status}）`);
    } else {
      console.log(`  ⚠️  ${r.proxy.id} 不可用（${r.latencyMs}ms${r.error ? `, ${r.error}` : ''}）`);
    }
  }
  return results
    .filter((r) => r.alive)
    .sort((a, b) => a.latencyMs - b.latencyMs)
    .map((r) => r.proxy);
}

function parseProxyArg(raw) {
  return raw.split(',').map((pair) => {
    const idx = pair.indexOf('=');
    if (idx <= 0) {
      throw new Error(`--proxies 格式应为 id=prefix，收到: "${pair}"`);
    }
    return { id: pair.slice(0, idx), prefix: pair.slice(idx + 1) };
  });
}

async function main() {
  const args = process.argv.slice(2);
  const noProbe = args.includes('--no-probe');
  const proxyArgIdx = args.indexOf('--proxies');
  let proxies = DEFAULT_PROXIES;
  if (proxyArgIdx >= 0 && args[proxyArgIdx + 1]) {
    proxies = parseProxyArg(args[proxyArgIdx + 1]);
  }
  // --notes-file <path>：release notes markdown，写入 latest.json 的 notes 字段
  const notesFileIdx = args.indexOf('--notes-file');
  let notes;
  if (notesFileIdx >= 0 && args[notesFileIdx + 1]) {
    const notesPath = args[notesFileIdx + 1];
    if (!fs.existsSync(notesPath)) {
      console.error(`--notes-file 指定的文件不存在: ${notesPath}`);
      process.exit(1);
    }
    notes = readFile(notesPath);
    console.log(`notes: 使用 ${notesPath}（${notes.length} 字符）`);
  }
  // 位置参数 = 去掉各选项及其值后的前三个
  const optionValue = new Set(['--proxies', '--notes-file']);
  const stripped = args.filter((_, i) => {
    if (args[i] === '--no-probe' || optionValue.has(args[i])) return false;
    return !optionValue.has(args[i - 1]);
  });
  const [version, artifactsDir, outputFile] = stripped;

  if (!version || !artifactsDir || !outputFile) {
    console.error('Usage: node scripts/generate-latest-json.js <version> <artifacts-dir> <output-file> [--notes-file <path>] [--no-probe] [--proxies id=prefix,...]');
    process.exit(1);
  }

  const outDir = path.dirname(outputFile);

  // 1. 直连清单（全球用户默认通道）
  const latest = buildLatestJson(version, artifactsDir, '', notes);
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(outputFile, JSON.stringify(latest, null, 2));
  console.log(`Generated ${outputFile} with platforms: ${Object.keys(latest.platforms).join(', ')}`);

  // 2. 代理探测 → 每个存活代理生成一份镜像清单（探测失败不影响直连清单）
  let alive = [];
  if (noProbe) {
    console.log('--no-probe: 跳过代理探测，仅生成直连清单。');
  } else {
    console.log('探测 GitHub 加速代理可用性…');
    alive = await probeProxies(proxies);
    if (alive.length === 0) {
      console.warn('⚠️  全部代理探测失败：本次仅发布直连 latest.json（国内用户需自行配置网络加速）。');
    }
  }

  for (const proxy of alive) {
    const mirrorFile = path.join(outDir, `latest-mirror-${proxy.id}.json`);
    const mirror = buildLatestJson(version, artifactsDir, proxy.prefix, notes);
    fs.writeFileSync(mirrorFile, JSON.stringify(mirror, null, 2));
    console.log(`Generated ${mirrorFile} (via ${proxy.prefix})`);
  }

  // 3. 汇总提示（镜像清单需要随 Release 上传；endpoints 已按直连 + 代理顺序配置）
  if (alive.length > 0) {
    console.log('提示：请将以上 latest-mirror-*.json 与 latest.json 一并上传到 GitHub Release。');
    console.log('tauri.conf.json updater.endpoints 已按「直连 + 代理镜像」顺序配置，客户端自动回退。');
  }
}

main();
