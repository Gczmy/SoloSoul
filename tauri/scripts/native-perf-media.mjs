#!/usr/bin/env node
/** RF-312：显式媒体行程，不改变既有 v1 输入行程。 */
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { main } from './native-perf-sdk-journey.mjs';
import { safeError } from './native-perf-run.mjs';
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  main(process.argv.slice(2), 'sdk-media')
    .then((code) => {
      process.exitCode = code;
    })
    .catch((error) => {
      process.stderr.write(safeError(error) + '\n');
      process.exitCode = 1;
    });
