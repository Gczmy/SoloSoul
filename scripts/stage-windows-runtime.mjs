#!/usr/bin/env node
// App-local release CRT deployment uses Visual Studio's redistributable directory,
// never System32 or a debug CRT. See docs/release_process.md.
import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { verifyWindowsPdfium as verifyX64Dll } from "./verify-windows-pdfium.mjs";

const REQUIRED_CRT = [
  "msvcp140.dll",
  "msvcp140_1.dll",
  "vcruntime140.dll",
  "vcruntime140_1.dll",
];
const compareVersion = (a, b) => b.localeCompare(a, "en", { numeric: true });

export function findCrtDirectory(redistRoot) {
  const architectureDir = path.join(redistRoot, "x64");
  const entries = fs
    .readdirSync(architectureDir, { withFileTypes: true })
    .filter(
      (entry) =>
        entry.isDirectory() && /^Microsoft\.VC\d+\.CRT$/.test(entry.name),
    )
    .map((entry) => entry.name)
    .sort(compareVersion);
  if (!entries.length)
    throw new Error("Visual Studio x64 release CRT directory not found");
  return path.join(architectureDir, entries[0]);
}

function installedCrtDirectory() {
  if (process.env.VCToolsRedistDir)
    return findCrtDirectory(process.env.VCToolsRedistDir);
  const vswhere = path.join(
    process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)",
    "Microsoft Visual Studio",
    "Installer",
    "vswhere.exe",
  );
  const result = spawnSync(
    vswhere,
    [
      "-latest",
      "-products",
      "*",
      "-requires",
      "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
      "-property",
      "installationPath",
    ],
    { encoding: "utf8", windowsHide: true },
  );
  if (result.error || result.status !== 0 || !result.stdout.trim())
    throw new Error(
      "Visual Studio C++ redistributable location is unavailable",
    );
  const root = path.join(result.stdout.trim(), "VC", "Redist", "MSVC");
  const versions = fs
    .readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && /^\d+(\.\d+)+$/.test(entry.name))
    .map((entry) => entry.name)
    .sort(compareVersion);
  if (!versions.length)
    throw new Error("Visual Studio C++ redistributable version not found");
  return findCrtDirectory(path.join(root, versions[0]));
}

export function stageWindowsRuntime({ crtDir, runtimeDir, outputDir }) {
  const dllNames = fs
    .readdirSync(crtDir, { withFileTypes: true })
    .filter(
      (entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".dll"),
    )
    .map((entry) => entry.name);
  for (const required of REQUIRED_CRT)
    if (!dllNames.some((name) => name.toLowerCase() === required))
      throw new Error(`Missing release CRT: ${required}`);
  const sources = dllNames.map((name) => ({
    name,
    source: path.join(crtDir, name),
  }));
  sources.push({
    name: "DirectML.dll",
    source: path.join(runtimeDir, "DirectML.dll"),
  });
  // Validate the complete input set before replacing any staged file.
  const files = sources.map(({ name, source }) => {
    if (!fs.lstatSync(source).isFile())
      throw new Error(`Runtime DLL must be a regular file: ${name}`);
    verifyX64Dll(source);
    const bytes = fs.readFileSync(source);
    return {
      name,
      bytes: bytes.length,
      sha256: createHash("sha256").update(bytes).digest("hex"),
    };
  });
  fs.mkdirSync(outputDir, { recursive: true });
  if (fs.lstatSync(outputDir).isSymbolicLink())
    throw new Error("Runtime stage cannot be a link");
  const previous = fs.readdirSync(outputDir, { withFileTypes: true });
  if (
    previous.some(
      (entry) =>
        !entry.isFile() ||
        !(/\.dll$/i.test(entry.name) || entry.name === "windows-runtime.json"),
    )
  )
    throw new Error("Unexpected content in managed runtime stage");
  const names = new Set(files.map((file) => file.name.toLowerCase()));
  for (const entry of previous)
    if (/\.dll$/i.test(entry.name) && !names.has(entry.name.toLowerCase()))
      fs.unlinkSync(path.join(outputDir, entry.name));
  for (const { name, source } of sources)
    fs.copyFileSync(source, path.join(outputDir, name));
  const manifest = {
    schema: 1,
    architecture: "x64",
    files: files.sort((a, b) => a.name.localeCompare(b.name)),
  };
  fs.writeFileSync(
    path.join(outputDir, "windows-runtime.json"),
    JSON.stringify(manifest, null, 2) + "\n",
  );
  return manifest;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
) {
  try {
    if (process.platform !== "win32")
      throw new Error("Windows runtime staging requires a native Windows host");
    const projectRoot = path.resolve(
      path.dirname(fileURLToPath(import.meta.url)),
      "..",
    );
    const tauriDir = path.join(projectRoot, "tauri");
    const targetDir = path.resolve(
      tauriDir,
      process.env.CARGO_TARGET_DIR || "target",
    );
    const profile =
      process.env.TAURI_ENV_DEBUG === "true" ? "debug" : "release";
    const explicitTarget =
      process.env.CARGO_BUILD_TARGET || process.env.TAURI_ENV_TARGET_TRIPLE;
    const candidates = [path.join(targetDir, profile)];
    if (explicitTarget)
      candidates.unshift(path.join(targetDir, explicitTarget, profile));
    const runtimeDir = candidates.find((dir) =>
      fs.existsSync(path.join(dir, "DirectML.dll")),
    );
    if (!runtimeDir)
      throw new Error(
        "Built DirectML.dll is missing; build the Windows application first",
      );
    const manifest = stageWindowsRuntime({
      crtDir: installedCrtDirectory(),
      runtimeDir,
      outputDir: path.join(
        tauriDir,
        "src-tauri",
        "resources-desktop",
        "windows-runtime",
      ),
    });
    console.log(
      `[stage-windows-runtime] Staged ${manifest.files.length} validated x64 release DLLs`,
    );
  } catch (error) {
    console.error(`[stage-windows-runtime] ${error.message}`);
    process.exitCode = 1;
  }
}
