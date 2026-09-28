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
    // RF301 的试点仍来自真实函数，但不继承后来扩展的完整迁移选择。
    await writeFile(
      path.join(root, 'src-tauri/ipc-contracts.json'),
      JSON.stringify({
        commands: [{ name: 'get_app_info', source: 'src-tauri/src/commands/system.rs' }],
        types: [],
        events: [],
        crates: [],
      }),
    );
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
        paths: { '@/*': ['./src/*'] },
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

// 只提取真实传输声明；编译真实 wrapper / session adapter，不复制它们的泛型算法。
async function copyTypedSources(root, { session = false } = {}) {
  await mkdir(path.join(root, 'src/lib'), { recursive: true });
  for (const file of ['typedIpc.ts', ...(session ? ['sessionRequests.ts'] : [])]) {
    await writeFile(
      path.join(root, 'src/lib', file),
      await readFile(path.join(workspaceRoot, 'src/lib', file)),
    );
  }
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
}

const objectSnapshotCommands = [
  'object_list',
  'object_get',
  'object_field_suggestions',
  'object_create',
  'object_update',
  'object_delete',
  'object_sync_with_template',
  'object_ignore_template_sync',
  'object_list_deprecated_fields',
  'object_trash_list',
  'snapshot_count_batch',
  'snapshot_get_data',
  'snapshot_list',
  'snapshot_rollback',
];

async function withProductionFixture(run) {
  const manifest = JSON.parse(await readFile(path.join(workspaceRoot, outputFiles[1]), 'utf8'));
  assert.equal(manifest.schemaVersion, 1);
  assert.equal(new Set(manifest.sources).size, manifest.sources.length);
  for (const command of objectSnapshotCommands) {
    assert.ok(manifest.commands.includes(command), `真实迁移清单缺少 ${command}`);
    assert.ok(!manifest.unmigratedCommands.includes(command));
  }
  const root = await mkdtemp(path.join(tmpdir(), 'solosoul-object-contract-'));
  try {
    for (const file of manifest.sources) {
      assert.equal(typeof file, 'string');
      assert.ok(!path.isAbsolute(file) && !file.includes('\\'));
      assert.ok(file.split('/').every((part) => part !== '' && part !== '.' && part !== '..'));
      await mkdir(path.dirname(path.join(root, file)), { recursive: true });
      await writeFile(path.join(root, file), await readFile(path.join(workspaceRoot, file)));
    }
    // Rust 输入与配置完全来自本次真实 manifest，不构造另一份对象/快照 DTO。
    const regenerated = await generateContracts({ root });
    assert.deepEqual(regenerated, manifest);
    await run(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
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
    await copyTypedSources(root);
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

test('RF302 real object and snapshot contracts reject invalid wire calls through direct and session invokers', async () => {
  await withProductionFixture(async (root) => {
    await copyTypedSources(root, { session: true });
    const probe = path.join(root, 'object-snapshot-contract-probe.ts');
    await writeFile(
      probe,
      `
import { createTypedInvoker, invokeTypedCommand } from './src/lib/typedIpc';
import { invokeCommand } from './src/lib/ipcClient';
import { createSessionRequests } from './src/lib/sessionRequests';
import type {
  ObjectSummary, ObjectData, FieldSuggestion, TemplateSyncResult,
  DeprecatedField, TrashItemSummary, JsonValue, SyncFieldChangeItem,
} from './src/lib/generated/ipcContracts';

export async function checkObjectSnapshotCalls() {
  const accountId = 'synthetic-account';
  const objectId = 'synthetic-object';
  const snapshotId = 'synthetic-snapshot';
  const summaries: ObjectSummary[] = await invokeTypedCommand('object_list', {
    accountId, filter: { includeDeleted: false, typeId: 'note', parentId: null },
  });
  const object: ObjectData | null = await invokeTypedCommand('object_get', { accountId, objectId });
  const suggestions: FieldSuggestion[] = await invokeTypedCommand('object_field_suggestions', {
    accountId, excludeObjectId: null,
  });
  const created: ObjectData = await invokeTypedCommand('object_create', {
    input: { accountId, name: '合成', typeId: 'note', properties: { nested: [null, true, 1] }, id: objectId },
  });
  const updated: ObjectData = await invokeTypedCommand('object_update', {
    objectId, input: { name: '合成更新', properties: null, sensitivityLevel: null },
  });
  const deleted: null = await invokeTypedCommand('object_delete', { objectId });
  const sync: TemplateSyncResult = await invokeTypedCommand('object_sync_with_template', { objectId, dryRun: true });
  const ignored: null = await invokeTypedCommand('object_ignore_template_sync', { objectId, hash: 'synthetic-hash' });
  const deprecated: DeprecatedField[] = await invokeTypedCommand('object_list_deprecated_fields', { objectId });
  const trash: TrashItemSummary[] = await invokeTypedCommand('object_trash_list', { accountId, since: null });
  const counts: Record<string, number> = await invokeTypedCommand('snapshot_count_batch', { objectIds: [objectId] });
  const history: JsonValue = await invokeTypedCommand('snapshot_get_data', { snapshotId });
  const snapshots = await invokeTypedCommand('snapshot_list', { objectId });
  const timestamp: number = snapshots[0].timestamp;
  const triggeredBy: string = snapshots[0].triggeredBy;
  const rollback: null = await invokeTypedCommand('snapshot_rollback', { snapshotId, objectId });

  const nullableTemplate: string | null = created.templateId;
  const optionalTags: string[] | undefined = created.tags;
  const labels: JsonValue = created.propertyLabels;
  const summaryLabels: JsonValue | undefined = summaries[0].propertyLabels;
  const summaryTags: string[] = summaries[0].tags;
  const partial = createTypedInvoker(invokeCommand);
  const factoryResult: ObjectData | null = await partial('object_get', { accountId, objectId });
  const ticket = createSessionRequests().begin('object', accountId);
  const sessionResult: ObjectData | null = await ticket.invokeTyped('object_get', { accountId, objectId });
  const sessionRollback: null = await ticket.invokeTyped('snapshot_rollback', { snapshotId, objectId });

  const typeChange: SyncFieldChangeItem = { kind: 'type', payload: { oldType: 'text', newType: 'number' } };
  const optionsChange: SyncFieldChangeItem = { kind: 'options' };
  const metadataChange: SyncFieldChangeItem = { kind: 'metadata', payload: { metadataKeys: ['format'] } };
  if (typeChange.kind === 'type') {
    const oldType: string = typeChange.payload.oldType;
    void oldType;
  }

  // @ts-expect-error object_get 真实 Rust 要求 accountId。
  void invokeTypedCommand('object_get', { objectId });
  // @ts-expect-error object_list 同样必须携带账户。
  void invokeTypedCommand('object_list', { filter: null });
  // @ts-expect-error 创建对象缺少必需 properties。
  void invokeTypedCommand('object_create', { input: { accountId, name: 'n', typeId: 'note' } });
  // @ts-expect-error 创建对象缺少必需 accountId。
  void invokeTypedCommand('object_create', { input: { name: 'n', typeId: 'note', properties: {} } });
  // @ts-expect-error 创建对象缺少必需 name。
  void invokeTypedCommand('object_create', { input: { accountId, typeId: 'note', properties: {} } });
  // @ts-expect-error 创建对象缺少必需 typeId。
  void invokeTypedCommand('object_create', { input: { accountId, name: 'n', properties: {} } });
  // @ts-expect-error 任意 JSON 值不包括 undefined。
  void invokeTypedCommand('object_create', { input: { accountId, name: 'n', typeId: 'note', properties: undefined } });
  // @ts-expect-error 乐观 ID 必须是字符串或 null。
  void invokeTypedCommand('object_create', { input: { accountId, name: 'n', typeId: 'note', properties: {}, id: 1 } });
  // @ts-expect-error object_update 仍要求 name 和 properties。
  void invokeTypedCommand('object_update', { objectId, input: { name: 'n' } });
  // @ts-expect-error filter.includeDeleted 来自真实 bool。
  void invokeTypedCommand('object_list', { accountId, filter: { includeDeleted: 'yes' } });
  // @ts-expect-error 同步命令没有 accountId wire 参数。
  void invokeTypedCommand('object_sync_with_template', { accountId, objectId, dryRun: true });
  // @ts-expect-error dryRun 不可省略。
  void invokeTypedCommand('object_sync_with_template', { objectId });
  // @ts-expect-error dryRun 必须为 boolean。
  void invokeTypedCommand('object_sync_with_template', { objectId, dryRun: 1 });
  // @ts-expect-error 废弃字段命令没有 accountId wire 参数。
  void invokeTypedCommand('object_list_deprecated_fields', { accountId, objectId });
  // @ts-expect-error snapshot_get_data 需要 snapshotId。
  void invokeTypedCommand('snapshot_get_data', { objectId });
  // @ts-expect-error snapshot_list 需要 objectId。
  void invokeTypedCommand('snapshot_list', { snapshotId });
  // @ts-expect-error snapshot_count_batch 需要字符串数组。
  void invokeTypedCommand('snapshot_count_batch', { objectIds: objectId });
  // @ts-expect-error snapshot_count_batch 不接受数字 ID。
  void invokeTypedCommand('snapshot_count_batch', { objectIds: [1] });
  // @ts-expect-error 回滚要求 objectId 和 snapshotId。
  void invokeTypedCommand('snapshot_rollback', { snapshotId });
  // @ts-expect-error 回滚成功仍是 null，不是完整对象。
  const wrongRollback: ObjectData = await invokeTypedCommand('snapshot_rollback', { snapshotId, objectId });
  // @ts-expect-error 删除成功不是 boolean。
  const wrongDelete: boolean = await invokeTypedCommand('object_delete', { objectId });
  // @ts-expect-error 对象查询可能未找到，不能丢掉 null。
  const definitelyObject: ObjectData = await invokeTypedCommand('object_get', { accountId, objectId });
  // @ts-expect-error 历史载荷可为标量/数组/null，不能伪装为固定字段对象。
  const fixedHistory: { name: string } = await invokeTypedCommand('snapshot_get_data', { snapshotId });
  // @ts-expect-error 真实 typed factory 同样禁止缺账户。
  void partial('object_get', { objectId });
  // @ts-expect-error 原会话 adapter 不得把必需参数重新放宽。
  void ticket.invokeTyped('object_get', { objectId });
  // @ts-expect-error 原会话 adapter 不得接受任意返回泛型。
  void ticket.invokeTyped<ObjectData>('object_get', { accountId, objectId });
  // @ts-expect-error 调用者不能选择任意返回类型。
  void invokeTypedCommand<ObjectData>('object_get', { accountId, objectId });
  // @ts-expect-error 相邻标签 type 必須有相应 payload。
  const missingPayload: SyncFieldChangeItem = { kind: 'type' };
  // @ts-expect-error type 标签不能使用 name 的 payload。
  const mismatchedPayload: SyncFieldChangeItem = { kind: 'type', payload: { oldName: 'a', newName: 'b' } };
  // @ts-expect-error unit options 分支不得带 payload。
  const unexpectedPayload: SyncFieldChangeItem = { kind: 'options', payload: {} };
  // @ts-expect-error metadataKeys 必须是字符串数组。
  const invalidMetadata: SyncFieldChangeItem = { kind: 'metadata', payload: { metadataKeys: [1] } };
  // @ts-expect-error 未登记的命令拼写错误不能派发。
  void invokeTypedCommand('object_gte', { accountId, objectId });

  void wrongRollback; void wrongDelete; void definitelyObject; void fixedHistory;
  void missingPayload; void mismatchedPayload; void unexpectedPayload; void invalidMetadata;
  return {
    summaries, object, suggestions, created, updated, deleted, sync, ignored, deprecated, trash,
    counts, history, timestamp, triggeredBy, rollback, nullableTemplate, optionalTags, labels,
    summaryLabels, summaryTags, factoryResult, sessionResult, sessionRollback,
    typeChange, optionsChange, metadataChange,
  };
}
`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    // 每个负例都必须实际触发错误；无关编译错误或未消费的指令均失败。
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(result.stdout, '');
    assert.equal(result.stderr, '');
  });
});
