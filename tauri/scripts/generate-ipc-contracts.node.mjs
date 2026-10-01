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
        paths: {
          '@/*': ['./src/*'],
          // 使用实际 SDK 声明，保留 Channel 的私有成员与序列化能力。
          '@tauri-apps/api/core': [
            path.join(workspaceRoot, 'node_modules/@tauri-apps/api/core.d.ts'),
          ],
        },
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

test('RF306 actual plugin contracts preserve SDK channels, resource handles and nullable wire payloads', async () => {
  await withProductionFixture(async (root) => {
    await copyTypedSources(root, { session: true });
    const probe = path.join(root, 'plugin-contract-probe.ts');
    await writeFile(
      probe,
      `
import { Channel, Resource } from '@tauri-apps/api/core';
import { createTypedInvoker, invokeTypedCommand } from './src/lib/typedIpc';
import { invokeCommand } from './src/lib/ipcClient';
import { createSessionRequests } from './src/lib/sessionRequests';
import type {
  ResourceId, PluginInstallProgress, PluginInstallResult, PluginEvent, PluginManifest,
  PluginResult, PluginResultPayload, PluginSession, PluginAuditEntry, PluginAuditAction, MarketPluginInfo,
} from './src/lib/generated/ipcContracts';
export async function checkPluginCalls() {
  const pluginId = 'synthetic-plugin', version = '1.0.0';
  const market: MarketPluginInfo[] = await invokeTypedCommand('plugin_list_all', {});
  await invokeTypedCommand('plugin_list_all', { tier: null });
  const installed: PluginManifest[] = await invokeTypedCommand('plugin_list_installed');
  const attachmentJson: string = await invokeTypedCommand('plugin_list_attachments');
  const operationId: ResourceId = await invokeTypedCommand('create_plugin_install');
  const resource = new Resource(operationId);
  const onProgress = new Channel<PluginInstallProgress>();
  onProgress.onmessage = (progress) => {
    const bytes: number | null = progress.totalBytes;
    // @ts-expect-error totalBytes 是 nullable。
    const guaranteed: number = progress.totalBytes;
    void bytes; void guaranteed;
  };
  const install: PluginInstallResult = await invokeTypedCommand('plugin_install', { pluginId, version, operationId: resource.rid, onProgress });
  const update: PluginInstallResult = await invokeTypedCommand('plugin_update', { pluginId, operationId: null, onProgress });
  await invokeTypedCommand('plugin_update', { pluginId, onProgress });
  const uninstalled: null = await invokeTypedCommand('plugin_uninstall', { pluginId });
  const channel = new Channel<PluginEvent>();
  channel.onmessage = (event) => {
    const requestId: string | null = event.requestId;
    const kind: string = event.eventType;
    // @ts-expect-error requestId 必须先缩窄。
    const guaranteed: string = event.requestId;
    void requestId; void kind; void guaranteed;
  };
  const result: PluginResult = await invokeTypedCommand('plugin_run', { pluginId, params: { name: 'synthetic' }, channel });
  const consent: null = await invokeTypedCommand('plugin_consent_response', { requestId: 'synthetic-request', approved: true, value: null });
  const dialog: null = await invokeTypedCommand('plugin_dialog_response', { requestId: 'synthetic-request' });
  const sessions: PluginSession[] = await invokeTypedCommand('plugin_list_sessions');
  const sessionId: string = sessions[0].sessionId, createdAt: number = sessions[0].createdAt;
  const audit: PluginAuditEntry[] = await invokeTypedCommand('plugin_audit_log', { limit: 50 });
  const refreshed: null = await invokeTypedCommand('plugin_update_registry');
  const opened: null = await invokeTypedCommand('plugin_open_output_file', { outputDir: 'synthetic-output', path: 'file.pdf' });
  const copied: null = await invokeTypedCommand('plugin_copy_output_file', { outputDir: 'synthetic-output', path: 'file.pdf', destDir: 'synthetic-dest', fileName: 'copy.pdf' });
  const installedAt: number = install.installedAt;
  const nullableVersion: string | null = market[0].registryEntry.latestVersion;
  const nullableAuthor: string | null = installed[0].author;
  const jsonResults: PluginResultPayload[] = [null, 'text', false, 1, ['nested'], { arbitrary: [true, null] }];
  const completed: PluginAuditAction = { action: 'plugin_run_completed', exit_code: 0 };
  const approved: PluginAuditAction = { action: 'consent_approved', field_id: 'synthetic-field' };
  const factory = createTypedInvoker(invokeCommand);
  const viaFactory: PluginResult = await factory('plugin_run', { pluginId, params: {}, channel });
  const ticket = createSessionRequests().begin('plugin', 'synthetic-account');
  const viaSession: PluginResult = await ticket.invokeTyped('plugin_run', { pluginId, params: {}, channel });
  // @ts-expect-error 安装版本必填。
  void invokeTypedCommand('plugin_install', { pluginId, onProgress });
  // @ts-expect-error 市场版本必须缩窄。
  void invokeTypedCommand('plugin_install', { pluginId, version: nullableVersion, onProgress });
  // @ts-expect-error 句柄不是 Resource 对象。
  void invokeTypedCommand('plugin_install', { pluginId, version, operationId: resource, onProgress });
  // @ts-expect-error 数字不能替代 Channel。
  void invokeTypedCommand('plugin_install', { pluginId, version, onProgress: operationId });
  // @ts-expect-error JSON 对象缺少 SDK Channel 私有成员及序列化能力。
  void invokeTypedCommand('plugin_install', { pluginId, version, onProgress: { id: 1, onmessage: () => {} } });
  // @ts-expect-error JSON 字符串不是 Channel。
  void invokeTypedCommand('plugin_install', { pluginId, version, onProgress: '__CHANNEL__:1' });
  // @ts-expect-error 安装 Channel 不能接运行事件。
  void invokeTypedCommand('plugin_install', { pluginId, version, onProgress: channel });
  // @ts-expect-error 运行 Channel 不能接安装进度。
  void invokeTypedCommand('plugin_run', { pluginId, params: {}, channel: onProgress });
  // @ts-expect-error params 只接受字符串。
  void invokeTypedCommand('plugin_run', { pluginId, params: { enabled: true }, channel });
  // @ts-expect-error update 没有 version 参数。
  void invokeTypedCommand('plugin_update', { pluginId, version, onProgress });
  // @ts-expect-error 工厂同样要求 SDK Channel。
  void factory('plugin_run', { pluginId, params: {}, channel: {} });
  // @ts-expect-error session 同样要求 params。
  void ticket.invokeTyped('plugin_run', { pluginId, channel });
  // @ts-expect-error consent 必须指定 approved。
  void invokeTypedCommand('plugin_consent_response', { requestId: 'synthetic-request' });
  // @ts-expect-error dialog value 为 string/null。
  void invokeTypedCommand('plugin_dialog_response', { requestId: 'synthetic-request', value: 1 });
  // @ts-expect-error Host 没有虚构的 plugin_cancel。
  void invokeTypedCommand('plugin_cancel', { operationId });
  // @ts-expect-error Webview 是原生注入参数。
  void invokeTypedCommand('create_plugin_install', { webview: 'main' });
  // @ts-expect-error attachments 仍是 JSON 字符串。
  const wrongAttachments: unknown[] = await invokeTypedCommand('plugin_list_attachments');
  // @ts-expect-error session.id 不存在。
  const wrongId: string = sessions[0].id;
  // @ts-expect-error 会话时间是数字。
  const wrongTime: string = sessions[0].createdAt;
  // @ts-expect-error 审计变体仍为 snake_case 字段。
  const wrongExit: PluginAuditAction = { action: 'plugin_run_completed', exitCode: 0 };
  // @ts-expect-error 审计字段为 field_id。
  const wrongField: PluginAuditAction = { action: 'consent_approved', fieldId: 'f' };
  // @ts-expect-error nullable 输出字段仍必须出现。
  const partialEvent: PluginEvent = { eventType: 'log', jsonData: '{}' };
  // @ts-expect-error ResourceId 是数字。
  const wrongResource: ResourceId = '42';
  // @ts-expect-error JSON 不包括函数。
  const nonJson: PluginResultPayload = () => {};
  // @ts-expect-error 注册表版本信息在 versions map 中。
  const legacyVersion: string = market[0].registryEntry.minCoreVersion;
  const extraArgs = { pluginId, version, onProgress, extra: true };
  // @ts-expect-error 命名变量同样禁止多余参数。
  void invokeTypedCommand('plugin_install', extraArgs);
  void wrongAttachments; void wrongId; void wrongTime; void wrongExit; void wrongField;
  void partialEvent; void wrongResource; void nonJson; void legacyVersion;
  return { market, installed, attachmentJson, install, update, uninstalled, result, consent, dialog,
    sessions, sessionId, createdAt, audit, refreshed, opened, copied, installedAt, nullableVersion,
    nullableAuthor, jsonResults, completed, approved, viaFactory, viaSession };
}
`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(result.stdout, '');
    assert.equal(result.stderr, '');
  });
});

test('RF303 real serde fixtures compile against generated LLM commands and stream events', async () => {
  await withProductionFixture(async (root) => {
    const manifest = await generateContracts({ root });
    const conversationCommands = [
      'llm_list_conversations',
      'llm_list_trash',
      'llm_get_conversation',
      'llm_save_conversation',
      'llm_soft_delete_conversation',
      'llm_restore_conversation',
      'llm_permanent_delete',
      'llm_rename_conversation',
      'llm_send_message_stream',
    ];
    for (const command of conversationCommands) {
      assert.ok(manifest.commands.includes(command));
      assert.ok(!manifest.unmigratedCommands.includes(command));
    }
    assert.ok(manifest.events.includes('llm-stream-chunk'));
    const fixture = JSON.parse(
      await readFile(
        path.join(workspaceRoot, 'src-tauri/src/commands/llm/contracts/fixtures.json'),
        'utf8',
      ),
    );
    await copyTypedSources(root);
    const probe = path.join(root, 'llm-fixture-probe.ts');
    await writeFile(
      probe,
      `import type { IpcEvents, Conversation, ConversationSummary, ChatContextSelectionInput } from './src/lib/generated/ipcContracts';
import { invokeTypedCommand } from './src/lib/typedIpc';
const complete = ${JSON.stringify(fixture.complete)} satisfies IpcEvents['llm-stream-chunk'];
const persistFailed = ${JSON.stringify(fixture.persistFailed)} satisfies IpcEvents['llm-stream-chunk'];
const upstreamFailed = ${JSON.stringify(fixture.upstreamFailed)} satisfies IpcEvents['llm-stream-chunk'];
const conversation = ${JSON.stringify(fixture.conversation)} satisfies Conversation;
const summary = ${JSON.stringify(fixture.summary)} satisfies ConversationSummary;
const none = ${JSON.stringify(fixture.none)} satisfies ChatContextSelectionInput;
const selection = ${JSON.stringify(fixture.publicProfile)} satisfies ChatContextSelectionInput;
const accountId = complete.accountId, conversationId = complete.conversationId;
export async function check() {
  const response: Conversation = await invokeTypedCommand('llm_get_conversation', { accountId, conversationId });
  const saved: null = await invokeTypedCommand('llm_save_conversation', { accountId, conversation });
  const args = { accountId, conversationId, providerId: 'synthetic-provider', messages: [] };
  const result: null = await invokeTypedCommand('llm_send_message_stream', { ...args, contextSelection: selection });
  // @ts-expect-error ordinary send does not accept credentials.
  void invokeTypedCommand('llm_send_message_stream', { ...args, apiKey: 'synthetic' });
  const withKey = { ...args, apiKey: 'synthetic' };
  // @ts-expect-error named variables do not bypass credential rejection.
  void invokeTypedCommand('llm_send_message_stream', withKey);
  const { accountId: a, ...noAccount } = complete;
  const { conversationId: c, ...noConversation } = complete;
  const { requestId: r, ...noRequest } = complete;
  const { sessionGeneration: g, ...noGeneration } = complete;
  const { error: e, ...noError } = complete;
  // @ts-expect-error account identity is required.
  const a1: IpcEvents['llm-stream-chunk'] = noAccount;
  // @ts-expect-error conversation identity is required.
  const c1: IpcEvents['llm-stream-chunk'] = noConversation;
  // @ts-expect-error request identity is required.
  const r1: IpcEvents['llm-stream-chunk'] = noRequest;
  // @ts-expect-error generation is required.
  const g1: IpcEvents['llm-stream-chunk'] = noGeneration;
  // @ts-expect-error nullable error is still required.
  const e1: IpcEvents['llm-stream-chunk'] = noError;
  void a; void c; void r; void g; void e; void a1; void c1; void r1; void g1; void e1;
  return { response, saved, result, complete, persistFailed, upstreamFailed, summary, none };
}
`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stdout + result.stderr);
  });
});

test('RF303 a real stream identity change fails check without overwriting generated outputs', async () => {
  await withProductionFixture(async (root) => {
    const before = await snapshot(root);
    const source = path.join(root, 'src-tauri/src/commands/llm/contracts.rs');
    const current = await readFile(source, 'utf8');
    const changed = current.replace('pub request_id: String,', 'pub correlation_id: String,');
    assert.notEqual(current, changed);
    await writeFile(source, changed);
    await assert.rejects(generateContracts({ root, check: true }), /IPC contract drift/);
    assert.deepEqual(await snapshot(root), before);
    await generateContracts({ root });
    const [typescript] = await snapshot(root);
    assert.match(typescript, /correlationId: string/);
    assert.doesNotMatch(
      typescript.match(/export type LlmStreamPayload = [\s\S]*?};/)[0],
      /requestId/,
    );
  });
});

test('RF304 real serde transfer fixtures and negative requests compile against generated contracts', async () => {
  await withProductionFixture(async (root) => {
    const manifest = await generateContracts({ root });
    const commands = [
      'backup_list',
      'backup_create',
      'backup_restore',
      'backup_delete',
      'export_get_scope_tree',
      'export_estimate_size',
      'export_execute',
      'export_get_attachments_batch',
      'import_parse_package',
      'import_decrypt_preview',
      'import_execute_advanced',
      'import_operations_list',
      'import_operation_get',
      'import_operation_resume',
      'export_document_preflight',
      'export_objects_document',
    ];
    for (const command of commands) {
      assert.ok(manifest.commands.includes(command));
      assert.ok(!manifest.unmigratedCommands.includes(command));
    }
    const fixture = JSON.parse(
      await readFile(
        path.join(workspaceRoot, 'src-tauri/src/commands/export_import/contracts/fixtures.json'),
        'utf8',
      ),
    );
    await copyTypedSources(root, { session: true });
    const probe = path.join(root, 'transfer-fixture-probe.ts');
    await writeFile(
      probe,
      `import type { BackupInfo, ImportResult, ImportOperationSummary, ImportPreview, DecryptedImportPreview, PageGroup, ExportEstimate, AttachmentInfo, ExportDocumentResult, AdvancedImportRequestInput, ExportRequestInput, ExportScopeInput } from './src/lib/generated/ipcContracts';
import { invokeTypedCommand } from './src/lib/typedIpc';
import { createSessionRequests } from './src/lib/sessionRequests';
const backup = ${JSON.stringify(fixture.backup)} satisfies BackupInfo;
const all = ${JSON.stringify(fixture.requestAll)} satisfies AdvancedImportRequestInput;
const none = ${JSON.stringify(fixture.requestNone)} satisfies AdvancedImportRequestInput;
const missing = ${JSON.stringify(fixture.requestMissing)} satisfies AdvancedImportRequestInput;
const exported = ${JSON.stringify(fixture.exportRequest)} satisfies ExportRequestInput;
const scope = ${JSON.stringify(fixture.scope)} satisfies ExportScopeInput;
const complete = ${JSON.stringify(fixture.complete)} satisfies ImportResult;
const partial = ${JSON.stringify(fixture.partial)} satisfies ImportResult;
const uncommitted = ${JSON.stringify(fixture.notCommitted)} satisfies ImportResult;
const operation = ${JSON.stringify(fixture.operation)} satisfies ImportOperationSummary;
const preview = ${JSON.stringify(fixture.preview)} satisfies ImportPreview;
const decrypted = ${JSON.stringify(fixture.decrypted)} satisfies DecryptedImportPreview;
const page = ${JSON.stringify(fixture.pageGroup)} satisfies PageGroup;
const estimate = ${JSON.stringify(fixture.estimate)} satisfies ExportEstimate;
const attachment = ${JSON.stringify(fixture.attachment)} satisfies AttachmentInfo;
const document = ${JSON.stringify(fixture.document)} satisfies ExportDocumentResult;
export async function check() {
  const accountId = 'synthetic-account', operationId = complete.operationId;
  const ticket = createSessionRequests().begin('transfer', accountId);
  const imported: ImportResult = await ticket.invokeTyped('import_execute_advanced', { accountId, req: missing });
  const resumed: ImportResult = await ticket.invokeTyped('import_operation_resume', { accountId, operationId, password: null, sourcePath: null });
  const path: string = await invokeTypedCommand('export_execute', { accountId, req: exported });
  const created: BackupInfo = await invokeTypedCommand('backup_create', { name: backup.name });
  const restored: number = await invokeTypedCommand('backup_restore', { backupId: backup.id });
  const deleted: null = await invokeTypedCommand('backup_delete', { backupId: backup.id });
  const listed: BackupInfo[] = await invokeTypedCommand('backup_list');
  const text: ExportDocumentResult = await invokeTypedCommand('export_objects_document', { objectIds: [], savePath: 'synthetic.txt', format: 'txt' });
  // @ts-expect-error Backup DTO is snake_case.
  const size = backup.sizeBytes;
  // @ts-expect-error locale default allows omission, never null.
  const nullLocale: AdvancedImportRequestInput = { ...all, locale: null };
  // @ts-expect-error serde enum is camelCase.
  const invalidStrategy: AdvancedImportRequestInput = { ...all, strategy: 'keep_both' };
  // @ts-expect-error strategy remains required.
  const missingStrategy: AdvancedImportRequestInput = { sourcePath: '', password: '' };
  // @ts-expect-error nullable outcome fields are still required.
  const badOutcome: ImportResult = { operationId: null, sessionGeneration: 7, status: 'complete', objectCount: 0, attachmentCount: 0, templateCount: 0, snapshotCount: 0, preferencesImported: false, attachmentFilesWritten: 0 };
  // @ts-expect-error status does not use success as a variant.
  const badStatus: ImportResult = { ...complete, status: 'success' };
  // @ts-expect-error selected IDs are arrays, not boolean flags.
  const invalidSelection: AdvancedImportRequestInput = { ...all, selectedAttachmentIds: false };
  // @ts-expect-error export attachments require an explicit array.
  const nullExport: ExportScopeInput = { ...scope, selectedAttachmentIds: null };
  // @ts-expect-error restore accepts backupId, not backup_id.
  void invokeTypedCommand('backup_restore', { backup_id: backup.id });
  // @ts-expect-error resume does not accept a fresh request or selection changes.
  void ticket.invokeTyped('import_operation_resume', { accountId, operationId, req: all });
  const freshOnResume = { accountId, operationId, req: all };
  // @ts-expect-error named variables cannot bypass top-level command arguments.
  void ticket.invokeTyped('import_operation_resume', freshOnResume);
  // @ts-expect-error app is a native Runtime injection.
  void invokeTypedCommand('import_parse_package', { app: 'main', filePath: preview.filePath });
  void size; void nullLocale; void invalidStrategy; void missingStrategy; void badOutcome; void badStatus; void invalidSelection; void nullExport;
  return { imported, resumed, path, created, restored, deleted, listed, text, none, partial, uncommitted, operation, decrypted, page, estimate, attachment, document };
}`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stdout + result.stderr);
  });
});

test('RF304 real default and outcome drift fail check without overwriting outputs', async () => {
  await withProductionFixture(async (root) => {
    const before = await snapshot(root);
    const source = path.join(root, 'src-tauri/src/commands/export_import/contracts.rs');
    const original = await readFile(source, 'utf8');
    for (const [from, to] of [
      ['#[serde(default = "default_locale")]', ''],
      ['pub attachment_files_written: usize,', 'pub files_written: usize,'],
      ['pub failure_stage: Option<ImportStage>,', 'pub failure_stage: ImportStage,'],
    ]) {
      const changed = original.replace(from, to);
      assert.notEqual(changed, original);
      await writeFile(source, changed);
      await assert.rejects(generateContracts({ root, check: true }), /IPC contract drift/);
      assert.deepEqual(await snapshot(root), before);
    }
    await writeFile(source, original);
    const commandSource = path.join(root, 'src-tauri/src/commands/export_import/import.rs');
    const command = await readFile(commandSource, 'utf8');
    const invalid = command.replace(
      'import_parse_package<R: tauri::Runtime>',
      'import_parse_package<R: unknown::Runtime>',
    );
    assert.notEqual(command, invalid);
    await writeFile(commandSource, invalid);
    await assert.rejects(generateContracts({ root, check: true }), /single tauri::Runtime/);
    assert.deepEqual(await snapshot(root), before);
  });
});

test('RF305 real serde sync fixtures compile and reject missing wire identity, nullability and UI fields', async () => {
  await withProductionFixture(async (root) => {
    const manifest = await generateContracts({ root });
    assert.equal(manifest.commands.length, 89);
    assert.equal(manifest.events.length, 11);
    const fixture = JSON.parse(
      await readFile(
        path.join(workspaceRoot, 'src-tauri/src/sync/contracts/fixtures.json'),
        'utf8',
      ),
    );
    await copyTypedSources(root, { session: true });
    const types = {
      peer: 'SyncPeer',
      status: 'SyncStatus',
      result: 'SyncResult',
      host: 'RecoveryHostInfo',
      discovered: 'DiscoveredDevice',
      recoveryComplete: 'ImportResultSummary',
      recoveryPartial: 'ImportResultSummary',
      recoveryNotCommitted: 'ImportResultSummary',
    };
    const declarations = Object.entries(types).map(
      ([key, type]) => `const ${key}=${JSON.stringify(fixture[key])} satisfies ${type};`,
    );
    for (const [key, event] of Object.entries({
      pairing: 'sync-pairing-request',
      completed: 'sync-completed',
      conflictsUpdated: 'sync-conflicts-updated',
      nsdFailed: 'sync-nsd-failed',
      deviceStart: 'device-sync-auto-status',
      deviceComplete: 'device-sync-auto-status',
      deviceError: 'device-sync-auto-status',
      progressManual: 'sync-progress',
      progressAutomatic: 'sync-progress',
      progressError: 'sync-progress',
      cloudStatus: 'cloud-sync-status',
      cloudIncoming: 'cloud-sync-incoming',
      recoveryDownload: 'recovery-progress',
      recoveryImport: 'recovery-progress',
      safRevoked: 'saf-auth-revoked',
    }))
      declarations.push(
        `const ${key}=${JSON.stringify(fixture[key])} satisfies IpcEvents['${event}'];`,
      );
    const probe = path.join(root, 'sync-fixture-probe.ts');
    await writeFile(
      probe,
      `import type {IpcEvents,SyncPeer,SyncStatus,SyncResult,RecoveryHostInfo,DiscoveredDevice,ImportResultSummary,JsonValue} from './src/lib/generated/ipcContracts';
import {invokeTypedCommand} from './src/lib/typedIpc';
import {createSessionRequests} from './src/lib/sessionRequests';
${declarations.join('\n')}
export async function check() {
const ticket=createSessionRequests().begin('sync','synthetic-account');
const status:SyncStatus=await ticket.invokeTyped('sync_get_status');
const sync:SyncResult=await ticket.invokeTyped('sync_with_device',{deviceId:'synthetic'});
const stop:null=await ticket.invokeTyped('sync_enable',{enable:false});
const host:RecoveryHostInfo=await invokeTypedCommand('recovery_host_start');
const rename:string|null=await ticket.invokeTyped('sync_rename_peer',{peerNodeId:'synthetic',name:''});
const discovered:DiscoveredDevice[]=await ticket.invokeTyped('mdns_discover',{timeoutMs:100});
const recovered:ImportResultSummary=await ticket.invokeTyped('recovery_restore_existing_from_host',{accountId:'synthetic-account',hostAddr:'synthetic',pin:'123456'});
const config:JsonValue=await ticket.invokeTyped('cloud_sync_get_config',{accountId:'synthetic-account'});
// @ts-expect-error Nullable customName is required on the wire.
const missingPeer:SyncPeer={id:'n',name:'n',addr:'a',fingerprint:'f',trusted:false,lastSeen:'',lastSeenTs:null,trustedAt:null,clientType:'unknown'};
const {sasCode,...noSas}=pairing;
// @ts-expect-error SAS exists on every actual pairing sender.
const missingPair:IpcEvents['sync-pairing-request']=noSas;
const {outboundRecords,...noOutbound}=completed;
// @ts-expect-error Completion includes both directions.
const missingCompleted:IpcEvents['sync-completed']=noOutbound;
const {sessionGeneration,...noGeneration}=cloudIncoming;
// @ts-expect-error Cloud generation is required.
const missingCloud:IpcEvents['cloud-sync-incoming']=noGeneration;
const {message,...noMessage}=deviceComplete;
// @ts-expect-error Terminal message is nullable but required.
const missingDevice:IpcEvents['device-sync-auto-status']=noMessage;
const {failureStage,...noStage}=recoveryComplete;
// @ts-expect-error Flatten keeps required nullable import fields.
const missingRecovery:ImportResultSummary=noStage;
// @ts-expect-error UI stamps never belong to wire SyncResult.
const stamped:SyncResult={...sync,at:42};
// @ts-expect-error HLC identity remains a hex string.
const badHlc:SyncResult={...sync,conflicts:[{...sync.conflicts[0],local_hlc:{wall_time_ms:1,counter:0,node_id:[0,1]}}]};
// @ts-expect-error Stop switch is boolean.
void ticket.invokeTyped('sync_enable',{enable:'false'});
// @ts-expect-error Ordinary sync does not receive a password.
void ticket.invokeTyped('sync_with_device',{deviceId:'synthetic',password:'synthetic'});
// @ts-expect-error get_config returns JSON, not an invented closed config DTO.
const wrongConfig:string=config.connectorType;
void missingPeer;void sasCode;void missingPair;void outboundRecords;void missingCompleted;void sessionGeneration;void missingCloud;void message;void missingDevice;void failureStage;void missingRecovery;void stamped;void badHlc;void wrongConfig;
return {status,sync,stop,host,rename,discovered,recovered,config};
}`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stdout + result.stderr);
  });
});
test('RF305 actual SAS, flattened recovery and platform-command drift cannot overwrite outputs', async () => {
  for (const [file, from, to] of [
    ['src-tauri/src/sync/contracts.rs', 'pub sas_code: String,', 'pub verification_code: String,'],
    ['src-tauri/src/sync/contracts.rs', '#[serde(flatten)]', ''],
    ['src-tauri/src/sync/ipc.rs', 'timeout_ms: u64,', 'timeout_ms: String,'],
  ])
    await withProductionFixture(async (root) => {
      const before = await snapshot(root),
        source = path.join(root, file),
        original = await readFile(source, 'utf8'),
        changed = original.replace(from, to);
      assert.notEqual(changed, original);
      await writeFile(source, changed);
      await assert.rejects(generateContracts({ root, check: true }), /IPC contract drift/);
      assert.deepEqual(await snapshot(root), before);
    });
});

test('RF307 Host error fixtures compile against actual rejection contracts and unsafe wire shapes fail', async () => {
  await withProductionFixture(async (root) => {
    const manifest = JSON.parse(await readFile(path.join(root, outputFiles[1]), 'utf8'));
    assert.deepEqual(manifest.structuredErrorCommands, [...objectSnapshotCommands].sort());
    await copyTypedSources(root);
    const fixtures = JSON.parse(
      await readFile(
        path.join(workspaceRoot, 'src-tauri/src/commands/object/tests/rf307-fixtures.json'),
        'utf8',
      ),
    );
    const declarations = Object.entries(fixtures).map(
      ([key, value]) => `const ${key}=${JSON.stringify(value)} satisfies BackendError;`,
    );
    const probe = path.join(root, 'error-fixture-probe.ts');
    await writeFile(
      probe,
      `import type {BackendError,IpcCommandErrors} from './src/lib/generated/ipcContracts';
${declarations.join('\n')}
const actual:IpcCommandErrors['object_create']=longName;
const rollback:IpcCommandErrors['snapshot_rollback']=rollbackMismatch;
const legacy:IpcCommandErrors['llm_get_conversation']='old string';
// @ts-expect-error migrated error is an object, not an English sentence.
const old:IpcCommandErrors['object_create']='Object not found';
// @ts-expect-error safeDetails is nullable but required.
const noDetails:BackendError={code:'OBJECT_NOT_FOUND',retryable:false};
// @ts-expect-error retryable is required.
const noRetry:BackendError={code:'OBJECT_NOT_FOUND',safeDetails:null};
// @ts-expect-error unknown error codes are not silently typed as any.
const unknown:BackendError={code:'FAKE_ERROR',safeDetails:null,retryable:true};
// @ts-expect-error safeDetails cannot carry arbitrary field values.
const unsafe:BackendError={code:'OBJECT_NAME_TOO_LONG',safeDetails:{stage:'validate',field:'secret'},retryable:false};
// @ts-expect-error stage is a real serialized enum.
const invalidStage:BackendError={code:'SNAPSHOT_INVALID',safeDetails:{stage:'snapshot_parse'},retryable:false};
// @ts-expect-error retryable is boolean.
const invalidRetry:BackendError={code:'OBJECT_READ_FAILED',safeDetails:null,retryable:'true'};
`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stdout + result.stderr);
  });
});

test('RF307 actual error serde and Result signature drift fail check without replacing generated files', async () => {
  await withProductionFixture(async (root) => {
    const before = await snapshot(root);
    for (const [file, oldText, newText] of [
      ['src-tauri/src/commands/error.rs', 'SCREAMING_SNAKE_CASE', 'camelCase'],
      ['src-tauri/src/commands/error.rs', 'pub retryable: bool', 'pub retryable: String'],
      [
        'src-tauri/src/commands/object/mod.rs',
        'Result<ObjectData, BackendError>',
        'Result<ObjectData, String>',
      ],
    ]) {
      const target = path.join(root, file);
      const original = await readFile(target, 'utf8');
      assert.ok(original.includes(oldText));
      await writeFile(target, original.replace(oldText, newText));
      await assert.rejects(generateContracts({ root, check: true }), /IPC contract drift/);
      assert.deepEqual(await snapshot(root), before);
      await writeFile(target, original);
    }
    await generateContracts({ root, check: true });
  });
});

test('RF908 guide retrieval requires the starting account at the actual typed IPC boundary', async () => {
  await withProductionFixture(async (root) => {
    await copyTypedSources(root, { session: true });
    const fixture = JSON.parse(
      await readFile(
        path.join(workspaceRoot, 'src-tauri/src/commands/llm/rf908-requests.json'),
        'utf8',
      ),
    );
    const probe = path.join(root, 'guide-account-probe.ts');
    await writeFile(
      probe,
      `import type {IpcCommands,GuideChunk} from './src/lib/generated/ipcContracts';
import {createSessionRequests} from './src/lib/sessionRequests';
const bound=${JSON.stringify(fixture.bound)} satisfies IpcCommands['llm_search_guide_chunks']['args'];
const empty=${JSON.stringify(fixture.empty)} satisfies IpcCommands['llm_search_guide_chunks']['args'];
const ticket=createSessionRequests().begin(undefined,bound.accountId);
const result:Promise<GuideChunk[]>=ticket.invokeTyped('llm_search_guide_chunks',bound);
void ticket.invokeTyped('llm_search_guide_chunks',{accountId:bound.accountId,query:'q',language:'zh-CN'});
void ticket.invokeTyped('llm_search_guide_chunks',{...bound,topK:null});
// @ts-expect-error Actual legacy body lacks the mandatory accountId.
void ticket.invokeTyped('llm_search_guide_chunks',${JSON.stringify(fixture.legacy)});
// @ts-expect-error topK is the actual integer input, never a string.
void ticket.invokeTyped('llm_search_guide_chunks',{...bound,topK:'3'});
// @ts-expect-error Returned GuideChunk fields are required.
const missing:GuideChunk={guideId:'id',chunkText:'text',similarity:0.8};
void empty; void result; void missing;
`,
    );
    const result = await compileProbe(root, probe);
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stdout + result.stderr);
  });
});
