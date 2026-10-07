#!/usr/bin/env node
/** 固定公开 PDF 首屏原生像素与观测耗时；不报告任意 PDF 加载完成。 */
import { main } from './native-perf-sdk-journey.mjs';
import { safeError } from './native-perf-run.mjs';
main(process.argv.slice(2), 'sdk-pdf-preview')
  .then((code) => {
    process.exitCode = code;
  })
  .catch((error) => {
    process.stderr.write(safeError(error) + '\n');
    process.exitCode = 1;
  });
