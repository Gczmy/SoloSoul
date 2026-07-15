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

const projectRoot = path.resolve(__dirname, '..');
const srcDir = path.join(projectRoot, '..', 'SoloSoul_plugin_market');
const destDir = path.join(projectRoot, 'src-tauri', 'resources-mobile', 'SoloSoul_plugin_market');

function copyFile(src, dst) {
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  fs.copyFileSync(src, dst);
}

function stagePluginMarket() {
  if (!fs.existsSync(srcDir)) {
    console.warn(`[stage-mobile-resources] 插件市场目录不存在，跳过: ${srcDir}`);
    return;
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

stagePluginMarket();
