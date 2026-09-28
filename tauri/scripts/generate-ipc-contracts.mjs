/** 只解析 Rust 源码；生成期间不调用 Tauri 或业务命令。 */
import { spawnSync } from 'node:child_process';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import * as prettier from 'prettier';

export const workspaceRoot = fileURLToPath(new URL('../', import.meta.url));
const outputs = {
  typescript: 'src/lib/generated/ipcContracts.ts',
  manifest: 'src/lib/generated/ipcContractManifest.json',
};

export function readRustContracts(root) {
  const result = spawnSync(
    'cargo',
    [
      'run',
      '--quiet',
      '--locked',
      '--manifest-path',
      path.join(workspaceRoot, 'Cargo.toml'),
      '-p',
      'solosoul-ipc-contract-gen',
      '--',
      '--root',
      root,
    ],
    { cwd: workspaceRoot, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `Rust contract generator failed (${result.status ?? result.signal}):\n${result.stderr}`,
    );
  }
  const payload = JSON.parse(result.stdout);
  if (
    typeof payload.typescript !== 'string' ||
    !payload.manifest ||
    payload.manifest.schemaVersion !== 1
  ) {
    throw new Error('Rust contract generator returned an invalid envelope');
  }
  return payload;
}

/** check 模式只比较文件，不自动修复漂移；Cargo 的编译缓存不属于生成契约。 */
export async function generateContracts({ root = workspaceRoot, check = false } = {}) {
  const payload = readRustContracts(path.resolve(root));
  const config = JSON.parse(await readFile(path.join(workspaceRoot, '.prettierrc'), 'utf8'));
  const formatted = {
    typescript: await prettier.format(payload.typescript, { ...config, parser: 'typescript' }),
    manifest: await prettier.format(JSON.stringify(payload.manifest), {
      ...config,
      parser: 'json',
    }),
  };
  const drift = [];
  for (const [key, relative] of Object.entries(outputs)) {
    const destination = path.join(root, relative);
    if (check) {
      let actual;
      try {
        actual = await readFile(destination, 'utf8');
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      if (actual !== formatted[key]) drift.push(relative);
    } else {
      await mkdir(path.dirname(destination), { recursive: true });
      await writeFile(destination, formatted[key], 'utf8');
    }
  }
  if (drift.length) {
    throw new Error(
      `IPC contract drift: ${drift.join(', ')}. Run npm run generate:contracts and review the diff.`,
    );
  }
  return payload.manifest;
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length > 1 || (args.length === 1 && args[0] !== '--check')) {
    throw new Error('Usage: node scripts/generate-ipc-contracts.mjs [--check]');
  }
  const check = args[0] === '--check';
  const manifest = await generateContracts({ check });
  console.log(
    `IPC contracts ${check ? 'verified' : 'generated'}: ${manifest.commands.length} migrated commands, ${manifest.unmigratedCommands.length} legacy commands, ${manifest.events.length} selected events.`,
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
