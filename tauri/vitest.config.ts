import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    // 主题测试需读取真实色板；其余组件样式继续使用默认 stub。
    css: { include: [/styles\/themes\.css/] },
    setupFiles: ['./src/test/setup.ts'],
    passWithNoTests: true,
    exclude: [
      'node_modules',
      'e2e',
      'dist',
      // 这些文件使用 node:test，由独立 Node 入口实际执行；Vitest 不转换它们。
      'scripts/native-perf-*.test.mjs',
      'scripts/android-native-regression.test.mjs',
      'scripts/update-distribution.test.js',
      'src-tauri/src/native_perf/observer.node.test.mjs',
    ],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      thresholds: {
        statements: 80,
        branches: 70,
        functions: 80,
        lines: 80,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
