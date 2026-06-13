#!/usr/bin/env node
/**
 * 根据构建产物生成 Tauri updater 所需的 latest.json。
 *
 * 用法：
 *   node scripts/generate-latest-json.js <version> <artifacts-dir> <output-file>
 *
 * 参数：
 *   - version: 应用版本号（例如 2.1.0）
 *   - artifacts-dir: 存放 .dmg/.exe/.AppImage 及对应 .sig 的目录
 *   - output-file: 输出的 latest.json 路径
 */

import fs from 'fs';
import path from 'path';

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

function buildLatestJson(version, artifactsDir) {
  const platforms = {};

  // macOS DMG (Apple Silicon)
  const macArmDmg = findFiles(artifactsDir, /SoloSoul_.*_aarch64\.dmg$/)[0];
  if (macArmDmg) {
    platforms['darwin-aarch64'] = {
      signature: resolveSignature(path.join(artifactsDir, macArmDmg)),
      url: `https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${macArmDmg}`,
    };
  }

  // macOS DMG (Intel)
  const macIntelDmg = findFiles(artifactsDir, /SoloSoul_.*_x64\.dmg$/)[0];
  if (macIntelDmg) {
    platforms['darwin-x86_64'] = {
      signature: resolveSignature(path.join(artifactsDir, macIntelDmg)),
      url: `https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${macIntelDmg}`,
    };
  }

  // Windows NSIS installer
  const windowsExe = findFiles(artifactsDir, /SoloSoul_.*_x64-setup\.exe$/)[0];
  if (windowsExe) {
    platforms['windows-x86_64'] = {
      signature: resolveSignature(path.join(artifactsDir, windowsExe)),
      url: `https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${windowsExe}`,
    };
  }

  // Linux AppImage
  const linuxAppImage = findFiles(artifactsDir, /SoloSoul_.*\.AppImage$/)[0];
  if (linuxAppImage) {
    platforms['linux-x86_64'] = {
      signature: resolveSignature(path.join(artifactsDir, linuxAppImage)),
      url: `https://github.com/Gczmy/SoloSoul/releases/download/v${version}/${linuxAppImage}`,
    };
  }

  if (Object.keys(platforms).length === 0) {
    throw new Error(`No installer artifacts found in ${artifactsDir}`);
  }

  return {
    version,
    notes: `SoloSoul v${version}`,
    pub_date: new Date().toISOString(),
    platforms,
  };
}

function main() {
  const [version, artifactsDir, outputFile] = process.argv.slice(2);

  if (!version || !artifactsDir || !outputFile) {
    console.error('Usage: node scripts/generate-latest-json.js <version> <artifacts-dir> <output-file>');
    process.exit(1);
  }

  const latest = buildLatestJson(version, artifactsDir);
  fs.mkdirSync(path.dirname(outputFile), { recursive: true });
  fs.writeFileSync(outputFile, JSON.stringify(latest, null, 2));
  console.log(`Generated ${outputFile} with platforms: ${Object.keys(latest.platforms).join(', ')}`);
}

main();
