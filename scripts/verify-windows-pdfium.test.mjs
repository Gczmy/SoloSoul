import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { verifyWindowsPdfium } from './verify-windows-pdfium.mjs';

const helper = fileURLToPath(new URL('./verify-windows-pdfium.mjs', import.meta.url));
const builder = fileURLToPath(new URL('./build_windows_release.sh', import.meta.url));

// 最小 PE32+ x64 DLL fixture：完整 DOS/COFF/optional/section 头及一段原始数据。
function peDll() {
  const bytes = Buffer.alloc(1024);
  bytes.writeUInt16LE(0x5a4d, 0);
  bytes.writeUInt32LE(0x80, 0x3c);
  bytes.writeUInt32LE(0x4550, 0x80);
  bytes.writeUInt16LE(0x8664, 0x84);
  bytes.writeUInt16LE(1, 0x86);
  bytes.writeUInt16LE(240, 0x94);
  bytes.writeUInt16LE(0x2022, 0x96);
  bytes.writeUInt16LE(0x20b, 0x98);
  bytes.writeUInt32LE(512, 0x98 + 60);
  bytes.write('.text', 0x188);
  bytes.writeUInt32LE(512, 0x188 + 16);
  bytes.writeUInt32LE(512, 0x188 + 20);
  bytes.fill(0xc3, 512);
  return bytes;
}

function directory(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'solosoul-windows-pdfium-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}

test('接受完整 Windows x64 PE32+ DLL，CLI 同样通过', (t) => {
  const filename = path.join(directory(t), 'pdfium.dll');
  fs.writeFileSync(filename, peDll());
  assert.equal(verifyWindowsPdfium(filename).architecture, 'x64');
  assert.equal(spawnSync(process.execPath, [helper, filename]).status, 0);
});

test('拒绝只有 macOS dylib、空 DLL 和伪装成 DLL 的 Mach-O 文件', (t) => {
  const root = directory(t);
  const filename = path.join(root, 'pdfium.dll');
  fs.writeFileSync(path.join(root, 'libpdfium.dylib'), Buffer.from([0xcf, 0xfa, 0xed, 0xfe]));
  assert.throws(() => verifyWindowsPdfium(filename), /ENOENT/);
  fs.writeFileSync(filename, Buffer.alloc(0));
  assert.throws(() => verifyWindowsPdfium(filename), /非空/);
  fs.writeFileSync(filename, Buffer.alloc(512, 0xcf));
  assert.throws(() => verifyWindowsPdfium(filename), /MZ/);
});

test('拒绝 x86、ARM64、非 DLL、PE32、错误签名与截断 PE', (t) => {
  const filename = path.join(directory(t), 'pdfium.dll');
  const invalid = [
    [(bytes) => bytes.writeUInt16LE(0x14c, 0x84), /AMD64/],
    [(bytes) => bytes.writeUInt16LE(0xaa64, 0x84), /AMD64/],
    [(bytes) => bytes.writeUInt16LE(0x22, 0x96), /标记 DLL/],
    [(bytes) => bytes.writeUInt16LE(0x10b, 0x98), /PE32\+/],
    [(bytes) => bytes.writeUInt32LE(0, 0x80), /PE 签名/],
    [(bytes) => bytes.writeUInt32LE(0xfffffff0, 0x3c), /PE 头位置/],
    [(bytes) => bytes.writeUInt32LE(4096, 0x188 + 16), /section 数据越界/],
  ];
  for (const [mutate, expected] of invalid) {
    const bytes = peDll();
    mutate(bytes);
    fs.writeFileSync(filename, bytes);
    assert.throws(() => verifyWindowsPdfium(filename), expected);
  }
  fs.writeFileSync(filename, peDll().subarray(0, 200));
  assert.throws(() => verifyWindowsPdfium(filename), /截断/);
});

function releaseFixture(t, { dll, download = 'valid', platform = 'MINGW64_NT-10.0' } = {}) {
  const root = directory(t);
  const resources = 'tauri/src-tauri/resources/pdfium';
  for (const name of ['.git', 'scripts', 'bin', 'tauri/scripts', resources,
    'tauri/src-tauri/resources/models/all-MiniLM-L6-v2', 'tauri/src-tauri/resources/models/pp-ocr-v6-small',
    'tauri/target/release/bundle']) fs.mkdirSync(path.join(root, name), { recursive: true });
  const calls = path.join(root, 'calls.log');
  fs.writeFileSync(calls, '');
  fs.writeFileSync(path.join(root, 'tauri/package.json'), '{"version": "2.13.1"}\n');
  fs.copyFileSync(helper, path.join(root, 'scripts/verify-windows-pdfium.mjs'));
  fs.writeFileSync(path.join(root, 'valid.dll'), peDll());
  fs.writeFileSync(path.join(root, resources, 'libpdfium.dylib'), 'macOS-only resource');
  fs.writeFileSync(path.join(root, 'tauri/target/release/bundle/keep.txt'), 'previous installer');
  if (dll) fs.writeFileSync(path.join(root, resources, 'pdfium.dll'), dll);
  const executable = (name, source) => fs.writeFileSync(path.join(root, name), `#!/bin/bash\n${source}\n`, { mode: 0o755 });
  executable('bin/uname', 'printf "%s\\n" "$PDFIUM_TEST_PLATFORM"');
  executable('bin/cargo', 'exit 0');
  executable('bin/python3', 'printf "python3\\n" >> "$PDFIUM_TEST_CALLS"');
  executable('bin/python', 'printf "python\\n" >> "$PDFIUM_TEST_CALLS"');
  // 所有安装/构建命令均为记录调用的 stub，只创建测试用空安装包标记。
  executable('bin/npm', `printf 'npm %s\\n' "$*" >> "$PDFIUM_TEST_CALLS"
if [[ "$*" == "run tauri build" ]]; then
  mkdir -p target/release/bundle/nsis
  printf 'mock installer' > target/release/bundle/nsis/SoloSoul_2.13.1_x64-setup.exe
fi`);
  executable('tauri/scripts/download-pdfium.sh', `printf 'download\\n' >> "$PDFIUM_TEST_CALLS"
case "$PDFIUM_TEST_DOWNLOAD" in
  valid) cp "$PDFIUM_TEST_DLL" "${resources}/pdfium.dll" ;;
  invalid) printf 'not a DLL' > "${resources}/pdfium.dll" ;;
  mac-only) printf 'mac only' > "${resources}/libpdfium.dylib" ;;
  failure) exit 7 ;;
esac`);
  const run = () => {
    const result = spawnSync('bash', [builder], { cwd: root, encoding: 'utf8', timeout: 15000,
      env: { ...process.env, VERSION: '2.13.1', PATH: `${path.join(root, 'bin')}${path.delimiter}${process.env.PATH}`,
        PDFIUM_TEST_PLATFORM: platform, PDFIUM_TEST_DOWNLOAD: download,
        PDFIUM_TEST_CALLS: calls, PDFIUM_TEST_DLL: path.join(root, 'valid.dll') } });
    return { ...result, calls: fs.readFileSync(calls, 'utf8').trim().split('\n').filter(Boolean) };
  };
  return { root, run };
}

test('发布脚本不能被 macOS dylib 放行：自动下载并复验 x64 DLL 后才进入构建', (t) => {
  const result = releaseFixture(t).run();
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.equal(result.calls.filter((call) => call === 'download').length, 1);
  assert.ok(result.calls.indexOf('download') < result.calls.indexOf('npm ci'));
});

test('发布脚本接受现有 x64 DLL，无需重新下载', (t) => {
  const result = releaseFixture(t, { dll: peDll() }).run();
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.ok(!result.calls.includes('download'));
  assert.ok(result.calls.includes('npm run tauri build'));
});

test('下载失败或下载后仍无有效 DLL，必须在清理产物和构建之前终止', (t) => {
  for (const download of ['mac-only', 'invalid', 'failure']) {
    const fixture = releaseFixture(t, { download });
    const result = fixture.run();
    assert.notEqual(result.status, 0, download);
    assert.ok(result.calls.includes('download'));
    assert.ok(!result.calls.some((call) => call.startsWith('npm ')));
    assert.ok(fs.existsSync(path.join(fixture.root, 'tauri/target/release/bundle/keep.txt')));
  }
});

test('WSL/Linux 在下载依赖、安装和构建前明确拒绝', (t) => {
  const result = releaseFixture(t, { platform: 'Linux' }).run();
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /不支持 WSL/);
  assert.deepEqual(result.calls, []);
});
