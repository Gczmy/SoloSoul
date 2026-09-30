/** RF-312：复制已有本地 Evergreen Runtime 的隔离诊断工具；不是 Fixed Version CAB。 */
import path from 'node:path';
import { createHash } from 'node:crypto';
import { createReadStream, constants } from 'node:fs';
import {
  lstat,
  realpath,
  readdir,
  readFile,
  open,
  mkdir,
  copyFile,
  writeFile,
} from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const BS = String.fromCharCode(92);
const EXTENDED = BS + BS + '?' + BS;
const MAX_FILES = 3000;
const MAX_BYTES = 1024 * 1024 * 1024;
const MAX_MANIFEST_BYTES = 4 * 1024 * 1024;
export const CORE_FILES = Object.freeze([
  'msedgewebview2.exe',
  'msedge.dll',
  'EBWebView/x64/EmbeddedBrowserWebView.dll',
]);
const ordinal = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
const message = (error) => String(error?.message ?? error).slice(0, 2000);

function version(value) {
  if (
    typeof value !== 'string' ||
    !/^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(value) ||
    value.split('.').some((n) => Number(n) > 65535)
  )
    throw new Error('Expected an exact four-part Windows Runtime version');
  return value;
}
function ordinaryLocalPath(value) {
  if (
    typeof value !== 'string' ||
    !/^[A-Za-z]:/.test(value) ||
    value[2] !== BS ||
    !path.win32.isAbsolute(value) ||
    path.win32.normalize(value) !== value
  )
    throw new Error('Runtime paths must be ordinary absolute local-drive paths without traversal');
  return value;
}
function ordinary(value) {
  return ordinaryLocalPath(value.startsWith(EXTENDED) ? value.slice(4) : value);
}
function samePath(a, b) {
  return ordinary(a).toLowerCase() === ordinary(b).toLowerCase();
}
function canonicalRoot(owned) {
  if (
    owned?.schemaVersion !== 1 ||
    owned.scope !== 'windows-native-perf-owned' ||
    owned.nativePerfFeature !== true ||
    owned.preparationStatus !== 'ready' ||
    typeof owned.root !== 'string' ||
    !owned.root.startsWith(EXTENDED) ||
    !/^[a-f0-9]{32}$/.test(owned.runId ?? '') ||
    owned.identifier !== 'com.solosoul.rf312perf.' + owned.runId
  )
    throw new Error('Runtime staging requires the exact prepared canonical owned root/run');
  ordinary(owned.root);
  return owned.root;
}
export function validateRuntimeRelative(relative) {
  if (
    typeof relative !== 'string' ||
    !relative ||
    Buffer.byteLength(relative) > 1024 ||
    relative.includes(BS) ||
    relative.startsWith('/') ||
    relative.includes(':') ||
    relative.split('/').length > 64
  )
    throw new Error('Unsafe Runtime relative path');
  for (const part of relative.split('/')) {
    if (
      !part ||
      part === '.' ||
      part === '..' ||
      part.endsWith('.') ||
      part.trimEnd() !== part ||
      /[<>:"|?*\x00-\x1f]/.test(part) ||
      /^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])(?:\.|$)/i.test(part)
    )
      throw new Error('Unsafe Runtime relative path');
  }
  return relative;
}
async function regular(target, directory) {
  const stat = await lstat(target);
  if (stat.isSymbolicLink() || (directory ? !stat.isDirectory() : !stat.isFile()))
    throw new Error(
      'Runtime path is linked or not a regular ' + (directory ? 'directory' : 'file'),
    );
  const actual = ordinary(await realpath(target));
  if (!samePath(actual, target))
    throw new Error('Runtime physical path differs from its exact directory');
  return { stat, actual };
}
async function hashFile(file, expectedBytes) {
  const hash = createHash('sha256');
  let bytes = 0;
  for await (const chunk of createReadStream(file)) {
    bytes += chunk.length;
    if (bytes > expectedBytes) throw new Error('Runtime source file changed while hashing');
    hash.update(chunk);
  }
  if (bytes !== expectedBytes) throw new Error('Runtime source file changed while hashing');
  return hash.digest('hex');
}
async function inventory(folder) {
  const dirs = [];
  const entries = [];
  let total = 0;
  await regular(folder, true);
  async function visit(dir, prefix, depth) {
    if (depth > 64) throw new Error('Runtime tree depth exceeds its bounded inventory');
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const relative = validateRuntimeRelative(prefix ? prefix + '/' + entry.name : entry.name);
      const target = path.join(dir, entry.name);
      const stat = await lstat(target);
      if (stat.isSymbolicLink()) throw new Error('Runtime tree contains a link/reparse path');
      if (stat.isDirectory()) {
        if (dirs.length >= MAX_FILES)
          throw new Error('Runtime directory inventory exceeds 3000 entries');
        await regular(target, true);
        dirs.push(relative);
        await visit(target, relative, depth + 1);
      } else if (stat.isFile()) {
        if (
          entries.length >= MAX_FILES ||
          !Number.isSafeInteger(stat.size) ||
          (total += stat.size) > MAX_BYTES
        )
          throw new Error('Runtime inventory exceeds 3000 files or 1GiB');
        await regular(target, false);
        entries.push({ relative, bytes: stat.size });
      } else throw new Error('Runtime tree contains a non-regular entry');
    }
  }
  await visit(folder, '', 0);
  entries.sort((a, b) => ordinal(a.relative, b.relative));
  dirs.sort(ordinal);
  for (const entry of entries) {
    const file = path.join(folder, ...entry.relative.split('/'));
    entry.sha256 = await hashFile(file, entry.bytes);
    const current = await regular(file, false);
    if (current.stat.size !== entry.bytes)
      throw new Error('Runtime source file changed during inventory');
  }
  return { files: entries, directories: dirs };
}

// 固定只读命令：不 Add-Type、不查询其它进程命令行、不安装或选择系统 Runtime。
const INSPECTION_SCRIPT = String.raw`
$ErrorActionPreference = 'Stop'
try {
  $root = $env:SOLOSOUL_NATIVE_PERF_RUNTIME_SOURCE
  if ($env:SOLOSOUL_NATIVE_PERF_RUNTIME_INSPECTION -eq 'metadata') {
    # 明确使用当前 PS5.1 的系统 Security 模块，避免继承 PS7 PSModulePath 时选错。
    Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
  }
  $drive = [System.IO.DriveInfo]::new([System.IO.Path]::GetPathRoot($root))
  if ($drive.DriveType.ToString() -notin @('Fixed','Removable','Ram')) { throw 'Runtime source is not on a local disk' }
  $first = Get-Item -LiteralPath $root -Force
  if (-not $first.PSIsContainer -or ($first.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) { throw 'Runtime root is not a regular directory' }
  $pending = [System.Collections.Generic.Stack[string]]::new(); $pending.Push($root)
  $fileCount = 0; $dirCount = 0; $totalBytes = 0L
  while ($pending.Count) {
    foreach ($item in @(Get-ChildItem -LiteralPath $pending.Pop() -Force)) {
      if ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { throw 'Runtime contains a reparse path' }
      if ($item.PSIsContainer) { $dirCount++; if ($dirCount -gt 3000) { throw 'Too many Runtime directories' }; $pending.Push($item.FullName) }
      else { $fileCount++; $totalBytes += $item.Length; if ($fileCount -gt 3000 -or $totalBytes -gt 1073741824) { throw 'Runtime inventory exceeds limits' } }
    }
  }
  $cores = @()
  if ($env:SOLOSOUL_NATIVE_PERF_RUNTIME_INSPECTION -eq 'metadata') {
    foreach ($relative in @('msedgewebview2.exe','msedge.dll','EBWebView/x64/EmbeddedBrowserWebView.dll')) {
      $item = Get-Item -LiteralPath (Join-Path $root $relative) -Force
      $signature = Get-AuthenticodeSignature -LiteralPath $item.FullName
      $cores += [pscustomobject]@{ relative=$relative; fileVersion=$item.VersionInfo.FileVersion; productVersion=$item.VersionInfo.ProductVersion; signatureStatus=$signature.Status.ToString(); signerSubject=$signature.SignerCertificate.Subject; signerThumbprint=$signature.SignerCertificate.Thumbprint }
    }
  }
  [pscustomobject]@{ schemaVersion=1; scope='windows-native-perf-runtime-inspection'; sourceFolder=$first.FullName; fileCount=$fileCount; directoryCount=$dirCount; totalBytes=$totalBytes; coreFiles=@($cores) } | ConvertTo-Json -Depth 5 -Compress
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
`;
async function inspectWindows(folder, metadata) {
  if (process.platform !== 'win32') throw new Error('Runtime inspection requires Windows');
  const env = { ...process.env };
  env.SOLOSOUL_NATIVE_PERF_RUNTIME_SOURCE = ordinary(folder);
  env.SOLOSOUL_NATIVE_PERF_RUNTIME_INSPECTION = metadata ? 'metadata' : 'attributes';
  let result;
  try {
    result = await execFileAsync(
      'powershell.exe',
      ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command', INSPECTION_SCRIPT],
      { env, windowsHide: true, timeout: 30000, maxBuffer: 1024 * 1024 },
    );
  } catch (error) {
    throw new Error(
      'Read-only Runtime inspection failed: ' +
        (String(error.stderr ?? '')
          .trim()
          .slice(0, 2000) || String(error.code ?? 'unknown')),
    );
  }
  const value = JSON.parse(result.stdout.replace(/^\uFEFF/, '').trim());
  if (
    value.schemaVersion !== 1 ||
    value.scope !== 'windows-native-perf-runtime-inspection' ||
    !samePath(value.sourceFolder, folder) ||
    !Array.isArray(value.coreFiles)
  )
    throw new Error('Unexpected Runtime inspection result');
  return value;
}
export function validateCoreMetadata(value, expectedVersion) {
  version(expectedVersion);
  if (
    !CORE_FILES.includes(value?.relative) ||
    value.architecture !== 'AMD64' ||
    value.fileVersion !== expectedVersion ||
    value.productVersion !== expectedVersion ||
    value.signatureStatus !== 'Valid' ||
    typeof value.signerSubject !== 'string' ||
    !/(?:^|,\s*)O=Microsoft Corporation(?:\s*,|$)/i.test(value.signerSubject) ||
    !/^[a-f0-9]{40}$/i.test(value.signerThumbprint ?? '') ||
    !/^[a-f0-9]{64}$/.test(value.sha256 ?? '')
  )
    throw new Error(
      'Runtime core requires exact AMD64/version and Valid Microsoft Authenticode metadata',
    );
  return value;
}
export function validateAmd64Pe(header, fileBytes = header.length) {
  if (!Buffer.isBuffer(header) || header.length < 64 || header.readUInt16LE(0) !== 0x5a4d)
    throw new Error('Runtime core is not a PE image');
  const offset = header.readUInt32LE(60);
  if (
    offset < 64 ||
    offset > 1024 * 1024 ||
    offset + 264 > header.length ||
    offset + 264 > fileBytes ||
    header.readUInt32LE(offset) !== 0x4550 ||
    header.readUInt16LE(offset + 4) !== 0x8664 ||
    header.readUInt16LE(offset + 6) < 1 ||
    header.readUInt16LE(offset + 6) > 96 ||
    header.readUInt16LE(offset + 20) < 240 ||
    header.readUInt16LE(offset + 24) !== 0x20b
  )
    throw new Error('Runtime core is not a complete AMD64 PE32+ image');
  return 'AMD64';
}
async function readPe(file, bytes) {
  const handle = await open(file, 'r');
  try {
    const dos = Buffer.alloc(64);
    const first = await handle.read(dos, 0, 64, 0);
    if (first.bytesRead !== 64) throw new Error('Incomplete Runtime DOS header');
    const offset = dos.readUInt32LE(60);
    if (offset > 1024 * 1024 || offset < 64 || offset + 264 > bytes)
      throw new Error('Invalid Runtime PE offset');
    const header = Buffer.alloc(offset + 264);
    dos.copy(header);
    const next = await handle.read(header, offset, 264, offset);
    if (next.bytesRead !== 264) throw new Error('Incomplete Runtime PE header');
    return validateAmd64Pe(header, bytes);
  } finally {
    await handle.close();
  }
}
function verifyFiles(files) {
  if (!Array.isArray(files) || files.length < 3 || files.length > MAX_FILES)
    throw new Error('Runtime file inventory is incomplete or exceeds limits');
  const folded = new Set();
  let total = 0;
  let last = null;
  for (const file of files) {
    validateRuntimeRelative(file.relative);
    if (
      !Number.isSafeInteger(file.bytes) ||
      file.bytes < 0 ||
      (total += file.bytes) > MAX_BYTES ||
      !/^[a-f0-9]{64}$/.test(file.sha256 ?? '') ||
      folded.has(file.relative.toLowerCase()) ||
      (last !== null && ordinal(last, file.relative) >= 0)
    )
      throw new Error('Runtime file inventory is duplicated, unordered, changed or invalid');
    folded.add(file.relative.toLowerCase());
    last = file.relative;
  }
  return files;
}
function verifyDirectories(directories, files) {
  if (!Array.isArray(directories) || directories.length > 3000)
    throw new Error('Invalid bounded Runtime directory inventory');
  const seen = new Set();
  const exact = new Set();
  const fileNames = new Set(files.map((file) => file.relative.toLowerCase()));
  let last = null;
  for (const relative of directories) {
    validateRuntimeRelative(relative);
    if (
      relative.split('/').length > 64 ||
      seen.has(relative.toLowerCase()) ||
      fileNames.has(relative.toLowerCase()) ||
      (last !== null && ordinal(last, relative) >= 0)
    )
      throw new Error(
        'Runtime directory inventory is duplicated, unordered or conflicts with a file',
      );
    seen.add(relative.toLowerCase());
    exact.add(relative);
    last = relative;
  }
  for (const relative of [...directories, ...files.map((file) => file.relative)]) {
    const parts = relative.split('/');
    if (parts.length > 65) throw new Error('Runtime inventory exceeds maximum directory depth');
    for (let depth = 1; depth < parts.length; depth++)
      if (!exact.has(parts.slice(0, depth).join('/')))
        throw new Error('Runtime directory inventory is missing a parent directory');
  }
}
function verifyProof(proof) {
  if (proof?.sourceKind !== 'copied-local-evergreen')
    throw new Error('Runtime source must be copied-local-evergreen');
  ordinaryLocalPath(proof.sourceFolder);
  version(proof.expectedVersion);
  verifyFiles(proof.files);
  verifyDirectories(proof.directories, proof.files);
  if (!Array.isArray(proof.coreFiles) || proof.coreFiles.length !== CORE_FILES.length)
    throw new Error('Incomplete Runtime core metadata');
  const found = new Set();
  for (const core of proof.coreFiles) {
    validateCoreMetadata(core, proof.expectedVersion);
    if (
      found.has(core.relative) ||
      proof.files.find((file) => file.relative === core.relative)?.sha256 !== core.sha256
    )
      throw new Error('Runtime core hash does not match the exact inventory');
    found.add(core.relative);
  }
  return proof;
}
export function installedEvergreenFolder(
  expectedVersion,
  programFiles = process.env['ProgramFiles(x86)'],
) {
  version(expectedVersion);
  ordinaryLocalPath(programFiles);
  return path.win32.join(programFiles, 'Microsoft', 'EdgeWebView', 'Application', expectedVersion);
}
export async function inspectRuntimeSource(sourceAbs, expectedVersion) {
  ordinaryLocalPath(sourceAbs);
  version(expectedVersion);
  const installed = installedEvergreenFolder(expectedVersion);
  if (!samePath(sourceAbs, installed))
    throw new Error('Runtime source must be this exact installed Evergreen version directory');
  const sourceFolder = (await regular(sourceAbs, true)).actual;
  if (!samePath(sourceFolder, installed))
    throw new Error('Installed Evergreen physical source differs from its version directory');
  const metadata = await inspectWindows(sourceFolder, true);
  const tree = await inventory(sourceFolder);
  const coreFiles = [];
  for (const relative of CORE_FILES) {
    const file = tree.files.find((item) => item.relative === relative);
    const values = metadata.coreFiles.filter((item) => item.relative === relative);
    if (!file || values.length !== 1)
      throw new Error('Required Runtime core file is missing or ambiguous');
    const architecture = await readPe(path.join(sourceFolder, ...relative.split('/')), file.bytes);
    coreFiles.push(
      validateCoreMetadata({ ...values[0], sha256: file.sha256, architecture }, expectedVersion),
    );
  }
  const after = await inventory(sourceFolder);
  if (JSON.stringify(after) !== JSON.stringify(tree))
    throw new Error('Runtime source changed during inspection');
  const proof = verifyProof({
    sourceKind: 'copied-local-evergreen',
    sourceFolder,
    expectedVersion,
    files: tree.files,
    directories: tree.directories,
    coreFiles,
  });
  return proof;
}
async function newJson(file, value) {
  const text = JSON.stringify(value, null, 2) + '\n';
  if (Buffer.byteLength(text) > MAX_MANIFEST_BYTES)
    throw new Error('Runtime manifest exceeds 4MiB');
  await writeFile(file, text, { flag: 'wx' });
}
export async function stageRuntimeCopy(proof, owned) {
  let phase = 'ownership';
  let evidenceRoot;
  try {
    const root = canonicalRoot(owned);
    await regular(root, true);
    const publishedOwned = path.win32.join(root, 'native-perf-owned.json');
    await regular(publishedOwned, false);
    if (
      JSON.stringify(JSON.parse(await readFile(publishedOwned, 'utf8'))) !== JSON.stringify(owned)
    )
      throw new Error('Runtime owner differs from its published ownership manifest');
    evidenceRoot = root;
    phase = 'source-validation';
    verifyProof(proof);
    const runtimeFolder = path.win32.join(root, 'runtime');
    for (const [parent, child] of [
      [ordinary(root), proof.sourceFolder],
      [proof.sourceFolder, ordinary(root)],
    ]) {
      const relative = path.win32.relative(parent, child);
      if (
        !relative ||
        (!relative.startsWith('..' + BS) && relative !== '..' && !path.win32.isAbsolute(relative))
      )
        throw new Error('Runtime source and owned root must be independent directories');
    }
    await inspectWindows(proof.sourceFolder, false);
    const before = await inventory(proof.sourceFolder);
    if (
      JSON.stringify(before.files) !== JSON.stringify(proof.files) ||
      JSON.stringify(before.directories) !== JSON.stringify(proof.directories)
    )
      throw new Error('Runtime source changed since its inspection');
    for (const relative of CORE_FILES) {
      const file = proof.files.find((item) => item.relative === relative);
      await readPe(path.join(proof.sourceFolder, ...relative.split('/')), file.bytes);
    }
    phase = 'exclusive-copy';
    await mkdir(runtimeFolder);
    for (const directory of [...before.directories].sort(
      (a, b) => a.split('/').length - b.split('/').length || ordinal(a, b),
    ))
      await mkdir(path.win32.join(runtimeFolder, ...directory.split('/')));
    for (const file of before.files) {
      const source = path.join(proof.sourceFolder, ...file.relative.split('/'));
      await regular(source, false);
      await copyFile(
        source,
        path.win32.join(runtimeFolder, ...file.relative.split('/')),
        constants.COPYFILE_EXCL,
      );
    }
    phase = 'after-copy-verification';
    await inspectWindows(proof.sourceFolder, false);
    await inspectWindows(runtimeFolder, false);
    const after = await inventory(proof.sourceFolder);
    const copied = await inventory(runtimeFolder);
    if (
      JSON.stringify(before) !== JSON.stringify(after) ||
      JSON.stringify(before) !== JSON.stringify(copied)
    )
      throw new Error('Complete Runtime source/copy tree changed during staging');
    const manifest = {
      schemaVersion: 1,
      scope: 'windows-native-perf-copied-runtime',
      root,
      runId: owned.runId,
      sourceKind: proof.sourceKind,
      sourceFolder: proof.sourceFolder,
      expectedVersion: proof.expectedVersion,
      runtimeFolder,
      files: proof.files,
      directories: proof.directories,
      coreFiles: proof.coreFiles,
    };
    phase = 'exclusive-manifest';
    await newJson(path.win32.join(root, 'native-perf-runtime.json'), manifest);
    return manifest;
  } catch (error) {
    if (evidenceRoot) {
      try {
        await newJson(path.win32.join(evidenceRoot, 'native-perf-runtime-failure.json'), {
          schemaVersion: 1,
          scope: 'windows-native-perf-runtime-staging-failure',
          root: evidenceRoot,
          runId: owned.runId,
          phase,
          reason: message(error),
          performanceSample: false,
        });
      } catch {}
    }
    throw new Error('Runtime staging failed at ' + phase + ': ' + message(error));
  }
}
function verifyManifest(manifest) {
  if (
    manifest?.schemaVersion !== 1 ||
    manifest.scope !== 'windows-native-perf-copied-runtime' ||
    typeof manifest.root !== 'string' ||
    !manifest.root.startsWith(EXTENDED) ||
    !/^[a-f0-9]{32}$/.test(manifest.runId ?? '') ||
    manifest.runtimeFolder !== path.win32.join(manifest.root, 'runtime')
  )
    throw new Error('Invalid copied Runtime ownership manifest');
  ordinary(manifest.root);
  verifyProof(manifest);
  return manifest;
}
export function checkSelectedRuntimeMarker(value, manifest, rootPid, port, manifestSha256) {
  verifyManifest(manifest);
  const executable = manifest.files.find((file) => file.relative === 'msedgewebview2.exe');
  if (
    value?.schemaVersion !== 1 ||
    value.scope !== 'windows-native-perf-selected-runtime' ||
    value.mode !== 'copied-local-evergreen' ||
    value.performanceSample !== false ||
    value.root !== manifest.root ||
    value.runId !== manifest.runId ||
    !Number.isInteger(rootPid) ||
    rootPid < 1 ||
    value.pid !== rootPid ||
    !Number.isInteger(port) ||
    port < 1 ||
    port > 65535 ||
    value.port !== port ||
    value.runtimeFolder !== manifest.runtimeFolder ||
    value.browserExecutableFolder !== ordinary(manifest.runtimeFolder) ||
    value.sourceFolder !== manifest.sourceFolder ||
    value.expectedVersion !== manifest.expectedVersion ||
    value.availableVersion !== manifest.expectedVersion ||
    !/^[a-f0-9]{64}$/.test(manifestSha256 ?? '') ||
    value.manifestSha256 !== manifestSha256 ||
    value.executableSha256 !== executable.sha256
  )
    throw new Error('Selected Runtime marker differs from its exact manifest/owned run/version');
  return value;
}
export async function checkSelectedRuntimeBrowser(identity, processDiagnostic, manifest) {
  verifyManifest(manifest);
  const records = processDiagnostic?.processes?.filter((row) => row.pid === identity?.pid);
  if (
    processDiagnostic?.schemaVersion !== 1 ||
    processDiagnostic.scope !== 'windows-native-perf-process-diagnostics' ||
    processDiagnostic.mode !== 'live' ||
    processDiagnostic.success !== true ||
    processDiagnostic.identityVerified !== true ||
    records?.length !== 1 ||
    records[0].identityMatched !== true ||
    records[0].parentPid !== identity.parentPid ||
    records[0].creationMs !== identity.creationMs
  )
    throw new Error('Selected Runtime browser lacks a fresh verified process identity');
  const actual = records[0];
  const expected = path.win32.join(manifest.runtimeFolder, 'msedgewebview2.exe');
  if (
    !samePath(identity.executablePath, expected) ||
    !samePath(actual.executablePath, expected) ||
    actual.version?.reason !== null ||
    actual.version.fileVersion !== manifest.expectedVersion ||
    actual.version.productVersion !== manifest.expectedVersion
  )
    throw new Error('Selected Runtime browser fell back or has an unexpected actual version/path');
  const result = await regular(actual.executablePath, false);
  const expectedFile = manifest.files.find((file) => file.relative === 'msedgewebview2.exe');
  if (!samePath(result.actual, expected) || result.stat.size !== expectedFile.bytes)
    throw new Error('Selected Runtime browser physical file differs from its owned copy');
  const sha256 = await hashFile(result.actual, expectedFile.bytes);
  if (sha256 !== expectedFile.sha256)
    throw new Error('Selected Runtime browser executable hash changed');
  return {
    schemaVersion: 1,
    scope: 'windows-native-perf-selected-browser-proof',
    matched: true,
    pid: identity.pid,
    creationMs: identity.creationMs,
    executablePath: result.actual,
    bytes: result.stat.size,
    sha256,
    fileVersion: actual.version.fileVersion,
    productVersion: actual.version.productVersion,
    expectedVersion: manifest.expectedVersion,
    runtimeFolder: manifest.runtimeFolder,
    sourceKind: manifest.sourceKind,
  };
}
