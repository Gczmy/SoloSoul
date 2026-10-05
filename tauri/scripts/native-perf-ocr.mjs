#!/usr/bin/env node
/** 固定公开图片的真实原生选择器和首次 OCR 行程。 */
import { main } from './native-perf-sdk-journey.mjs';
import { safeError } from './native-perf-run.mjs';
main(process.argv.slice(2), 'sdk-ocr')
  .then((code) => {
    process.exitCode = code;
  })
  .catch((error) => {
    process.stderr.write(safeError(error) + '\n');
    process.exitCode = 1;
  });
