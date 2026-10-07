#!/usr/bin/env node
/** 固定公开PDF的原生能力诊断。保持失败原件，不报告PDF性能。 */
import { main } from './native-perf-sdk-journey.mjs';
import { safeError } from './native-perf-run.mjs';
main(process.argv.slice(2), 'sdk-pdf-diagnostic')
  .then((code) => {
    process.exitCode = code;
  })
  .catch((error) => {
    process.stderr.write(safeError(error) + '\n');
    process.exitCode = 1;
  });
