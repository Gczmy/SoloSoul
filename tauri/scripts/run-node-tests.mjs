#!/usr/bin/env node
/** RF1060：独立运行 Node 测试；平台跳过由具体 case 报告，禁止整文件跳过。 */
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const files = [
  'scripts/android-native-regression.test.mjs',
  'scripts/native-perf-run.test.mjs',
  'scripts/native-perf-diagnose.test.mjs',
  'scripts/native-perf-runtime.test.mjs',
  'scripts/native-perf-sdk-cdp.test.mjs',
  'scripts/native-perf-ui-root.node.test.mjs',
  'scripts/native-perf-ui-gate.node.test.mjs',
  'scripts/native-perf-sdk-journey.test.mjs',
  'scripts/native-perf-startup.test.mjs',
  'scripts/native-perf-memory.test.mjs',
  'src-tauri/src/native_perf/observer.node.test.mjs',
  'scripts/update-distribution.test.js',
  'scripts/stage-mobile-resources.test.cjs',
];

if (process.argv.length !== 2) {
  console.error('Independent Node tests use a fixed explicit file list and accept no arguments.');
  process.exitCode = 2;
} else {
  const result = spawnSync(process.execPath, ['--test', ...files], {
    cwd: fileURLToPath(new URL('../', import.meta.url)),
    stdio: 'inherit',
    windowsHide: true,
  });
  if (result.error || result.signal || result.status === null) {
    console.error(
      'Independent Node test process failed:',
      result.error?.message ?? result.signal ?? 'missing exit status',
    );
    process.exitCode = 1;
  } else {
    process.exitCode = result.status;
  }
}
