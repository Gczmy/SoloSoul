import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import ts from 'typescript';
import { generateContracts, workspaceRoot } from './generate-ipc-contracts.mjs';

const sourceFiles = [
  'src-tauri/src/lib.rs',
  'src-tauri/src/commands/mod.rs',
  'src-tauri/src/commands/system.rs',
  'src-tauri/permissions/solo-soul/default.toml',
  'src-tauri/ipc-contracts.json',
];
const outputFiles = [
  'src/lib/generated/ipcContracts.ts',
  'src/lib/generated/ipcContractManifest.json',
];

async function withFixture(run) {
  const root = await mkdtemp(path.join(tmpdir(), 'solosoul-ipc-contract-'));
  try {
    for (const file of sourceFiles) {
      await mkdir(path.dirname(path.join(root, file)), { recursive: true });
      await writeFile(path.join(root, file), await readFile(path.join(workspaceRoot, file)));
    }
    await run(root);
  } finally {
    // root 来自本测试的 mkdtemp，清理不会触及工作目录。
    await rm(root, { recursive: true, force: true });
  }
}

async function snapshot(root) {
  return Promise.all(outputFiles.map((file) => readFile(path.join(root, file), 'utf8')));
}

// 临时项目仅编译生成契约与实际 wrapper，不依赖应用的 Node/React ambient types。
async function compileProbe(root, probe) {
  const config = path.join(root, 'tsconfig.contracts.json');
  await writeFile(
    config,
    JSON.stringify({
      compilerOptions: {
        noEmit: true,
        strict: true,
        skipLibCheck: true,
        target: 'ES2022',
        module: 'ESNext',
        moduleResolution: 'bundler',
        types: [],
      },
      files: [probe],
    }),
  );
  return spawnSync(
    process.execPath,
    [path.join(workspaceRoot, 'node_modules/typescript/bin/tsc'), '--project', config],
    { cwd: root, encoding: 'utf8' },
  );
}

test('two real-source generations are byte-identical; check leaves outputs unchanged', async () => {
  await withFixture(async (root) => {
    const manifest = await generateContracts({ root });
    assert.deepEqual(manifest.commands, ['get_app_info']);
    assert.ok(manifest.unmigratedCommands.length > 0);
    assert.deepEqual(manifest.events, []);
    const first = await snapshot(root);
    await generateContracts({ root });
    assert.deepEqual(await snapshot(root), first);
    await generateContracts({ root, check: true });
    assert.deepEqual(await snapshot(root), first);
  });
});

test('an actual Rust command argument change fails check, and generated arguments reject the old TS call', async () => {
  await withFixture(async (root) => {
    await generateContracts({ root });
    const before = await snapshot(root);
    const file = path.join(root, 'src-tauri/src/commands/system.rs');
    const source = await readFile(file, 'utf8');
    const changed = source.replace('fn get_app_info()', 'fn get_app_info(information_key: String)');
    assert.notEqual(source, changed);
    await writeFile(file, changed);
    await assert.rejects(generateContracts({ root, check: true }), /IPC contract drift/);
    assert.deepEqual(await snapshot(root), before);
    await generateContracts({ root });
    const [typescript] = await snapshot(root);
    assert.match(typescript, /informationKey: string/);
    // 使用真实生成的 args；省略新增参数必须由 TypeScript 报错，而不是比较另一份手写 map。
    const probe = path.join(root, 'contract-probe.ts');
    await writeFile(
      probe,
      `import type { IpcCommands } from './src/lib/generated/ipcContracts';\nconst args: IpcCommands['get_app_info']['args'] = {};\nvoid args;\n`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    assert.equal(result.status, 2, result.stdout + result.stderr);
    assert.match(result.stdout, /TS2741.*informationKey/);
  });
});

test('missing output fails check without writing files; Rust parsing errors also fail generation', async () => {
  await withFixture(async (root) => {
    await assert.rejects(generateContracts({ root, check: true }), /IPC contract drift/);
    await assert.rejects(readFile(path.join(root, outputFiles[0])), { code: 'ENOENT' });
    await generateContracts({ root });
    const before = await snapshot(root);
    await writeFile(path.join(root, 'src-tauri/src/commands/system.rs'), 'this is not Rust');
    await assert.rejects(generateContracts({ root }), /Rust contract generator failed/);
    assert.deepEqual(await snapshot(root), before);
  });
});

test('the actual typed wrapper preserves command and argument correlation across generated commands', async () => {
  await withFixture(async (root) => {
    const systemFile = path.join(root, 'src-tauri/src/commands/system.rs');
    await writeFile(
      systemFile,
      `${await readFile(systemFile, 'utf8')}
#[tauri::command]
pub fn rf301_set_enabled(enabled: bool) -> Result<bool, String> {
    Ok(enabled)
}
`,
    );
    const libFile = path.join(root, 'src-tauri/src/lib.rs');
    const libSource = await readFile(libFile, 'utf8');
    const registered = libSource.replace(
      'commands::system::get_app_info,',
      'commands::system::get_app_info,\n        commands::system::rf301_set_enabled,',
    );
    assert.notEqual(registered, libSource);
    await writeFile(libFile, registered);
    const aclFile = path.join(root, 'src-tauri/permissions/solo-soul/default.toml');
    const aclSource = await readFile(aclFile, 'utf8');
    const allowed = aclSource.replace('"get_app_info",', '"get_app_info",\n  "rf301_set_enabled",');
    assert.notEqual(allowed, aclSource);
    await writeFile(aclFile, allowed);
    const selectionFile = path.join(root, 'src-tauri/ipc-contracts.json');
    const selection = JSON.parse(await readFile(selectionFile, 'utf8'));
    selection.commands.push({
      name: 'rf301_set_enabled',
      source: 'src-tauri/src/commands/system.rs',
    });
    await writeFile(selectionFile, JSON.stringify(selection));
    const manifest = await generateContracts({ root });
    assert.deepEqual(manifest.commands, ['get_app_info', 'rf301_set_enabled']);

    // 两条契约都由实际 Rust AST 生成；编译实际 wrapper，不复制其条件类型算法。
    await writeFile(
      path.join(root, 'src/lib/typedIpc.ts'),
      await readFile(path.join(workspaceRoot, 'src/lib/typedIpc.ts')),
    );
    const transportSource = ts.createSourceFile(
      'ipcClient.ts',
      await readFile(path.join(workspaceRoot, 'src/lib/ipcClient.ts'), 'utf8'),
      ts.ScriptTarget.Latest,
      true,
      ts.ScriptKind.TS,
    );
    const options = transportSource.statements.filter(
      (node) => ts.isInterfaceDeclaration(node) && node.name.text === 'InvokeOptions',
    );
    const invocations = transportSource.statements.filter(
      (node) => ts.isFunctionDeclaration(node) && node.name?.text === 'invokeCommand',
    );
    assert.equal(options.length, 1, '真实 InvokeOptions 必须能唯一提取');
    assert.equal(invocations.length, 1, '真实 invokeCommand 必须能唯一提取');
    const invocation = invocations[0];
    assert.ok(invocation.type, '传输函数必须有显式返回类型');
    // 这里仅移除实际传输函数体及 async，保留其真实泛型、参数、返回类型和选项声明。
    // 原生调用与鉴权运行时由 typedIpc.test.ts 的真实 ipcClient 链路验证。
    const declaration = ts.factory.updateFunctionDeclaration(
      invocation,
      [
        ts.factory.createModifier(ts.SyntaxKind.ExportKeyword),
        ts.factory.createModifier(ts.SyntaxKind.DeclareKeyword),
      ],
      invocation.asteriskToken,
      invocation.name,
      invocation.typeParameters,
      invocation.parameters,
      invocation.type,
      undefined,
    );
    const signature = ts
      .createPrinter()
      .printNode(ts.EmitHint.Unspecified, declaration, transportSource);
    await writeFile(
      path.join(root, 'src/lib/ipcClient.d.ts'),
      `${options[0].getText(transportSource)}\n${signature}\n`,
    );
    const probe = path.join(root, 'correlated-contract-probe.ts');
    await writeFile(
      probe,
      `
import { invokeTypedCommand } from './src/lib/typedIpc';
import type { AppInfo } from './src/lib/generated/ipcContracts';

export async function checkCorrelatedCalls(
  command: 'get_app_info' | 'rf301_set_enabled',
  independentArgs: undefined | { enabled: boolean },
  correlated:
    | [command: 'get_app_info']
    | [command: 'rf301_set_enabled', args: { enabled: boolean }],
) {
  const app: AppInfo = await invokeTypedCommand('get_app_info');
  const enabled: boolean = await invokeTypedCommand('rf301_set_enabled', { enabled: true });
  const options = { requireUnlocked: true, requestIsCurrent: () => true };
  const appWithOptions: AppInfo = await invokeTypedCommand('get_app_info', undefined, options);
  const withOptions: boolean = await invokeTypedCommand('rf301_set_enabled', { enabled: false }, options);
  const completeCall = ['rf301_set_enabled', { enabled: true }, options] as const;
  const tupleResult: boolean = await invokeTypedCommand(...completeCall);
  const unionResult: AppInfo | boolean = await invokeTypedCommand(...correlated);

  // @ts-expect-error 必需参数不能省略。
  void invokeTypedCommand('rf301_set_enabled');
  // @ts-expect-error 必需参数不能为 undefined。
  void invokeTypedCommand('rf301_set_enabled', undefined);
  // @ts-expect-error 字段类型必须匹配实际 Rust bool。
  void invokeTypedCommand('rf301_set_enabled', { enabled: 'yes' });
  // @ts-expect-error 无参命令不能接收另一命令的参数。
  void invokeTypedCommand('get_app_info', { enabled: true });
  // @ts-expect-error 未缩窄命令可能要求参数。
  void invokeTypedCommand(command, undefined);
  // @ts-expect-error 未缩窄命令可能是不接受参数的 get_app_info。
  void invokeTypedCommand(command, { enabled: true });
  // @ts-expect-error 两个独立联合不建立命令与参数关联。
  void invokeTypedCommand(command, independentArgs);
  // @ts-expect-error 响应按命令推导，不能任选另一命令响应。
  const wrongApp: AppInfo = await invokeTypedCommand('rf301_set_enabled', { enabled: true });
  // @ts-expect-error 无参命令的返回值同样不能被宽化。
  const wrongBoolean: boolean = await invokeTypedCommand('get_app_info');
  // @ts-expect-error 调用者不能指定任意返回类型。
  void invokeTypedCommand<AppInfo>('get_app_info');
  // @ts-expect-error options 必须处于第三个参数位置。
  void invokeTypedCommand('rf301_set_enabled', options);
  // @ts-expect-error options 仍须符合实际传输声明。
  void invokeTypedCommand('rf301_set_enabled', { enabled: true }, { requestIsCurrent: () => 'yes' });

  if (command === 'get_app_info') {
    const narrowed: AppInfo = await invokeTypedCommand(command);
    void narrowed;
  } else {
    const narrowed: boolean = await invokeTypedCommand(command, { enabled: true });
    void narrowed;
  }
  void wrongApp;
  void wrongBoolean;
  return { app, enabled, appWithOptions, withOptions, tupleResult, unionResult };
}
`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    // 必须成功且消费全部 @ts-expect-error；仅出现任意编译错误不算通过。
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(result.stdout, '');
    assert.equal(result.stderr, '');
  });
});
