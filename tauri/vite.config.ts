import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  clearScreen: false,
  build: {
    rolldownOptions: {
      output: {
        // P015-R2: 拆分重共享 chunk，降低首次拉取成本——
        // ① markdown 生态（react-markdown/remark/rehype/unified/micromark/hast 等）
        //   与 ② framer-motion 分别独立成块。配合 UpdateBanner/navButtonCards 的
        //   动态化改造，markdown 不再进入入口与首页共享路径（仅在帮助/LLM 聊天/
        //   更新横幅/快速聊天真正用到时加载）；实测 index 432K→317K、
        //   PageContainer 561K→338K，markdown/motion vendor 均不在 index.html 启动链。
        //   注：因关闭递归并入，react-dom/@ungap 等通用依赖留在原 chunk。
        //   motion 组正则（framer-motion|motion- 前缀）修正后 motion-dom/motion-utils
        //   已被捕获：PageContainer 561K→338K→238K、motion-vendor 32K→122K。
        //   实测首导航闭包 361.6K→360.2K（仅 -1.4K）——共享组件（DropdownSelect 等）
        //   真实使用 framer-motion，motion-vendor 必然随首导航加载，纯 chunk 归属无法
        //   减总量；真杠杆在代码层替换共享组件动画（另议）。
        codeSplitting: {
          // includeDependenciesRecursively 仅顶层生效（group 级字段不存在，会被静默忽略）。
          // 默认 true 会把捕获模块的依赖递归并入组（react-markdown 依赖 react/
          // @ungap/structured-clone），而 react 是入口必需 → 入口被迫静态依赖整块随启动加载
          // （实测：所有 chunk 都反向依赖 markdown-vendor，index.html modulepreload 含它）。
          includeDependenciesRecursively: false,
          groups: [
            {
              name: 'markdown-vendor',
              // 只捕获真正 markdown 专属的包；react/@ungap/structured-clone 等通用依赖留在原位。
              test: /node_modules[\\/](react-markdown|remark-|rehype-|micromark|mdast-|hast-|unist-|unified|vfile|vfile-message|lowlight|refractor|highlight\.js|comma-separated-tokens|property-information|space-separated-tokens|stringify-entities|character-entities|decode-named-character-reference|ccount|bail|trough|extend|is-plain-obj|trim-lines|zwitch|longest-streak|markdown-table|escape-string-regexp|devlop|html-void-elements|html-whitespace|web-namespaces|estree-util-|style-to-object|style-to-jsx|inline-style-parser)[\\/]/,
            },
            {
              name: 'motion-vendor',
              // 前缀项必须用 [^\\/]*（允许 motion-dom/motion-utils 等包名带子名），
              // 旧的 (framer-motion|motion-)[\\/] 会漏掉 motion-dom/motion-utils。
              test: /node_modules[\\/](framer-motion|motion-)[^\\/]*[\\/]/,
            },
          ],
        },
      },
    },
  },
  server: {
    port: Number(process.env.SOLOSOUL_VITE_PORT) || 1420,
    strictPort: true,
    host: host || false,
    hmr: {
      protocol: 'ws',
      host: host || 'localhost',
      port: Number(process.env.SOLOSOUL_VITE_HMR_PORT) || 1421,
    },
    watch: {
      ignored: ['**/src-tauri/**', '**/target/**'],
    },
  },
});
