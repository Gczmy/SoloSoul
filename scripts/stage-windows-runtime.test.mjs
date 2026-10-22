import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  findCrtDirectory,
  stageWindowsRuntime,
} from "./stage-windows-runtime.mjs";

function dll(machine = 0x8664) {
  const bytes = Buffer.alloc(1024);
  bytes.writeUInt16LE(0x5a4d, 0);
  bytes.writeUInt32LE(128, 60);
  bytes.writeUInt32LE(0x4550, 128);
  bytes.writeUInt16LE(machine, 132);
  bytes.writeUInt16LE(1, 134);
  bytes.writeUInt16LE(240, 148);
  bytes.writeUInt16LE(0x2022, 150);
  bytes.writeUInt16LE(0x20b, 152);
  bytes.writeUInt32LE(512, 212);
  bytes.writeUInt32LE(512, 408);
  bytes.writeUInt32LE(512, 412);
  return bytes;
}

function fixture(t) {
  const prefix = path.join(fs.realpathSync(os.tmpdir()), "solosoul-runtime-");
  const root = fs.mkdtempSync(prefix);
  t.after(() => {
    assert.ok(fs.realpathSync(root).startsWith(prefix));
    fs.rmSync(root, { recursive: true, force: true });
  });
  const crtDir = path.join(root, "x64", "Microsoft.VC143.CRT");
  const runtimeDir = path.join(root, "build", "release");
  const outputDir = path.join(root, "stage");
  for (const dir of [crtDir, runtimeDir])
    fs.mkdirSync(dir, { recursive: true });
  for (const name of [
    "msvcp140.dll",
    "msvcp140_1.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
  ])
    fs.writeFileSync(path.join(crtDir, name), dll());
  fs.writeFileSync(path.join(runtimeDir, "DirectML.dll"), dll());
  return { root, crtDir, runtimeDir, outputDir };
}

test("stages complete x64 dependencies with byte hashes and a path-free manifest", (t) => {
  const f = fixture(t);
  assert.equal(findCrtDirectory(f.root), f.crtDir);
  const result = stageWindowsRuntime(f);
  assert.equal(result.files.length, 5);
  for (const item of result.files) {
    const source = path.join(
      item.name === "DirectML.dll" ? f.runtimeDir : f.crtDir,
      item.name,
    );
    assert.deepEqual(
      fs.readFileSync(path.join(f.outputDir, item.name)),
      fs.readFileSync(source),
    );
    assert.match(item.sha256, /^[a-f0-9]{64}$/);
  }
  const manifest = fs.readFileSync(
    path.join(f.outputDir, "windows-runtime.json"),
    "utf8",
  );
  assert.deepEqual(JSON.parse(manifest), result);
  assert.equal(manifest.includes(f.root), false);
});

test("missing required CRT or DirectML fails before replacing the previous stage", (t) => {
  const f = fixture(t);
  stageWindowsRuntime(f);
  const previous = fs.readFileSync(
    path.join(f.outputDir, "windows-runtime.json"),
  );
  fs.unlinkSync(path.join(f.crtDir, "msvcp140_1.dll"));
  assert.throws(() => stageWindowsRuntime(f), /Missing release CRT/);
  assert.deepEqual(
    fs.readFileSync(path.join(f.outputDir, "windows-runtime.json")),
    previous,
  );
  fs.writeFileSync(path.join(f.crtDir, "msvcp140_1.dll"), dll());
  fs.unlinkSync(path.join(f.runtimeDir, "DirectML.dll"));
  assert.throws(() => stageWindowsRuntime(f), /ENOENT/);
  assert.deepEqual(
    fs.readFileSync(path.join(f.outputDir, "windows-runtime.json")),
    previous,
  );
});

test("rejects wrong-architecture and truncated runtime DLLs before staging", (t) => {
  const f = fixture(t);
  const source = path.join(f.runtimeDir, "DirectML.dll");
  fs.writeFileSync(source, dll(0x14c));
  assert.throws(() => stageWindowsRuntime(f), /x64/);
  assert.equal(fs.existsSync(f.outputDir), false);
  fs.writeFileSync(source, Buffer.from("MZ"));
  assert.throws(() => stageWindowsRuntime(f), /DOS/);
  assert.equal(fs.existsSync(f.outputDir), false);
});

test("refreshes managed DLLs while rejecting unexpected nested content", (t) => {
  const f = fixture(t);
  stageWindowsRuntime(f);
  fs.writeFileSync(path.join(f.outputDir, "old-runtime.dll"), dll());
  stageWindowsRuntime(f);
  assert.equal(fs.existsSync(path.join(f.outputDir, "old-runtime.dll")), false);
  fs.mkdirSync(path.join(f.outputDir, "unexpected"));
  assert.throws(() => stageWindowsRuntime(f), /Unexpected content/);
  assert.equal(fs.existsSync(path.join(f.outputDir, "unexpected")), true);
});

test("does not accept a debug CRT directory as a redistributable source", (t) => {
  const f = fixture(t);
  fs.renameSync(f.crtDir, path.join(f.root, "x64", "Microsoft.VC143.DebugCRT"));
  assert.throws(() => findCrtDirectory(f.root), /release CRT/);
});
