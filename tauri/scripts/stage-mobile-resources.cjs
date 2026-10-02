#!/usr/bin/env node
/**
 * 为移动端构建生成精简后的插件市场资源目录。
 *
 * Tauri 的 bundle.resources 会原样把 `SoloSoul_plugin_market/plugins` 整个目录打进 APK，
 * 其中包含大量 `target/` 编译产物（.rlib、.rmeta、.dylib 等），在 Android 上完全用不到。
 * 该脚本只复制每个插件运行所需的最小文件：
 * - SoloSoul_plugin_market/registry.json
 * - SoloSoul_plugin_market/plugins/<id>/manifest.json
 * - SoloSoul_plugin_market/plugins/<id>/plugin.wasm
 *
 * 输出目录 `src-tauri/resources-mobile/` 已加入 .gitignore，不会污染源码。
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const projectRoot = path.resolve(__dirname, '..');
const srcDir = path.join(projectRoot, '..', 'SoloSoul_plugin_market');
const destDir = path.join(projectRoot, 'src-tauri', 'resources-mobile', 'SoloSoul_plugin_market');

function copyFile(src, dst) {
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  fs.copyFileSync(src, dst);
}

function stagePluginMarket() {
  if (!fs.existsSync(srcDir)) {
    throw new Error(`[stage-mobile-resources] 插件市场目录不存在: ${srcDir}`);
  }

  // 清理旧的移动端资源目录
  if (fs.existsSync(destDir)) {
    fs.rmSync(destDir, { recursive: true, force: true });
  }

  // 复制 registry.json
  const registrySrc = path.join(srcDir, 'registry.json');
  if (fs.existsSync(registrySrc)) {
    copyFile(registrySrc, path.join(destDir, 'registry.json'));
  }

  // 复制每个插件的 manifest.json 与 plugin.wasm
  const pluginsDir = path.join(srcDir, 'plugins');
  if (fs.existsSync(pluginsDir)) {
    for (const pluginId of fs.readdirSync(pluginsDir)) {
      const pluginSrcDir = path.join(pluginsDir, pluginId);
      if (!fs.statSync(pluginSrcDir).isDirectory()) continue;

      const pluginDestDir = path.join(destDir, 'plugins', pluginId);
      for (const fileName of ['manifest.json', 'plugin.wasm']) {
        const fileSrc = path.join(pluginSrcDir, fileName);
        if (fs.existsSync(fileSrc)) {
          copyFile(fileSrc, path.join(pluginDestDir, fileName));
        }
      }
    }
  }

  console.log(`[stage-mobile-resources] 已生成移动端插件市场资源: ${destDir}`);
}

/** 清单仅含实际运行资源；内容版本不依赖时间、机器路径或目录枚举顺序。 */
function createResourceManifest(roots) {
  const files = [];
  function visit(source, relative) {
    const stat = fs.lstatSync(source);
    if (stat.isSymbolicLink()) throw new Error(`内置资源不允许符号链接: ${relative}`);
    if (stat.isDirectory()) {
      for (const name of fs.readdirSync(source).sort())
        visit(path.join(source, name), `${relative}/${name}`);
    } else if (stat.isFile()) {
      if (
        relative.split('/').some((part) => !part || part === '.' || part === '..') ||
        /[\\\t\r\n\0]/.test(relative)
      ) {
        throw new Error(`非法资源路径: ${relative}`);
      }
      const bytes = fs.readFileSync(source);
      files.push({
        path: relative,
        size: bytes.length,
        sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
      });
    } else throw new Error(`非法资源类型: ${relative}`);
  }
  for (const [prefix, source] of Object.entries(roots)) visit(source, prefix);
  files.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  const content = files.map((file) => `${file.path}\0${file.size}\0${file.sha256}\n`).join('');
  return { schema: 1, version: crypto.createHash('sha256').update(content).digest('hex'), files };
}

function stageResourceManifest() {
  const manifest = createResourceManifest({
    docs: path.join(projectRoot, 'src-tauri/resources/docs'),
    SoloSoul_plugin_market: destDir,
  });
  for (const required of ['docs/guides/index.json', 'SoloSoul_plugin_market/registry.json']) {
    if (!manifest.files.some((file) => file.path === required))
      throw new Error(`缺少关键内置资源: ${required}`);
  }
  fs.writeFileSync(
    path.join(path.dirname(destDir), 'bundled-resource-manifest.json'),
    JSON.stringify(manifest, null, 2) + '\n',
  );
  console.log(
    `[stage-mobile-resources] 资源版本 ${manifest.version}，${manifest.files.length} 文件`,
  );
}

if (require.main === module) {
  stagePluginMarket();
  stageResourceManifest();
}
module.exports = { createResourceManifest };
