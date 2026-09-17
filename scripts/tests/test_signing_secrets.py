"""发行脚本的私钥边界回归；所有构建/签名工具均由临时桩替代。"""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


PROJECT_ROOT = Path(__file__).resolve().parents[2]
FAKE_KEY = "FAKE-UPDATER-KEY-ONLY-FOR-REGRESSION\nSECOND-FAKE-KEY-LINE"

TOOL_STUB = r'''
import hashlib
import json
import os
from pathlib import Path
import sys

tool = Path(sys.argv[0]).name
args = sys.argv[1:]
record = {"tool": tool, "argv": args}
if tool == "npx":
    if args[:3] != ["tauri", "signer", "sign"]:
        raise SystemExit("Unexpected npx invocation")
    key = os.environ.get("TAURI_SIGNING_PRIVATE_KEY", "")
    record["key_digest"] = hashlib.sha256(key.encode()).hexdigest()
    Path(args[-1] + ".sig").write_text("fake-signature")
elif tool == "npm":
    if args == ["ci"]:
        pass
    elif args[:3] == ["run", "tauri", "build"]:
        app = Path("target/release/bundle/macos/SoloSoul.app/Contents")
        app.mkdir(parents=True)
        (app / "placeholder").write_text("fake-app")
    else:
        raise SystemExit("Unexpected npm invocation")
elif tool == "node":
    if args != ["-v"]:
        raise SystemExit("Unexpected node invocation")
    print("v22.0.0")
elif tool == "uname":
    print("arm64")
elif tool == "create-dmg":
    Path(args[-2]).write_text("fake-dmg")
elif tool == "tar":
    if args[0] != "czf":
        raise SystemExit("Unexpected tar invocation")
    Path(args[1]).write_text("fake-archive")
elif tool not in {"codesign", "xattr"}:
    raise SystemExit("Unexpected tool invocation: " + tool)
with open(os.environ["SOLOSOUL_TEST_CALLS"], "a") as log:
    log.write(json.dumps(record) + "\n")
'''


class SigningSecretTests(unittest.TestCase):
    def run_fixture(self, script_name, *, key_source, trace=False, verbose=False):
        with tempfile.TemporaryDirectory(prefix="solosoul-signing-test-") as temporary:
            fixture = Path(temporary)
            tools = fixture / "tools"
            tools.mkdir()
            for tool in [
                "node", "npm", "npx", "cargo", "codesign", "xattr", "create-dmg", "tar", "uname"
            ]:
                executable = tools / tool
                executable.write_text(f"#!{sys.executable}\n" + TOOL_STUB)
                executable.chmod(0o755)

            scripts = fixture / "scripts"
            scripts.mkdir()
            fake_key_path = fixture / "signing" / "fake.key"
            fake_key_path.parent.mkdir()
            if key_source == "file":
                fake_key_path.write_text(FAKE_KEY + "\n")

            source = (PROJECT_ROOT / "scripts" / script_name).read_text()
            # 仅重定向源码中的固定文件路径；不改 HOME，绝不访问维护者的真实密钥。
            private_path = "${HOME}/SoloSoul/signing/tauri-updater/secret.key"
            self.assertIn(private_path, source)
            copied_script = scripts / script_name
            copied_script.write_text(source.replace(private_path, str(fake_key_path)))

            (fixture / ".git").mkdir()
            tauri = fixture / "tauri"
            tauri.mkdir()
            (tauri / "package.json").write_text('{"version": "1.2.3"}\n')
            resources = tauri / "src-tauri" / "resources"
            for model in ["all-MiniLM-L6-v2", "pp-ocr-v6-small"]:
                (resources / "models" / model).mkdir(parents=True)
            (resources / "pdfium").mkdir()
            (resources / "pdfium" / "fake.dylib").write_text("fake-pdfium")
            artifacts = fixture / "SoloSoul-Releases"
            artifacts.mkdir()
            (artifacts / "SoloSoul_1.2.3_arm64.app.tar.gz").write_text("fake-archive")

            calls_path = fixture / "calls.jsonl"
            # 环境白名单避免把运行测试机器上的真实签名变量传入子进程。
            env = {
                "PATH": str(tools) + os.pathsep + "/usr/bin:/bin",
                "SOLOSOUL_TEST_CALLS": str(calls_path),
                "HOME": os.environ.get("HOME", ""),
            }
            if key_source == "environment":
                env["TAURI_SIGNING_PRIVATE_KEY"] = FAKE_KEY
            command = ["/bin/bash"]
            if trace:
                command.append("-x")
            command.append(str(copied_script))
            if verbose:
                command.append("--verbose")
            result = subprocess.run(
                command, cwd=fixture, env=env, capture_output=True, text=True, timeout=30
            )
            calls = (
                [json.loads(line) for line in calls_path.read_text().splitlines()]
                if calls_path.exists()
                else []
            )

            for line in FAKE_KEY.splitlines():
                self.assertNotIn(line, result.stdout)
                self.assertNotIn(line, result.stderr)
                self.assertNotIn(line, json.dumps(calls))
            signer_calls = [call for call in calls if call["tool"] == "npx"]
            if key_source == "missing":
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(signer_calls, [])
                return

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(len(signer_calls), 1)
            self.assertEqual(
                signer_calls[0]["key_digest"], hashlib.sha256(FAKE_KEY.encode()).hexdigest()
            )
            self.assertNotIn("--private-key", signer_calls[0]["argv"])
            if verbose:
                build = next(
                    call
                    for call in calls
                    if call["tool"] == "npm"
                    and call["argv"][:3] == ["run", "tauri", "build"]
                )
                self.assertIn("--verbose", build["argv"])

    def test_builder_keeps_file_and_environment_keys_out_of_verbose_output(self):
        for key_source in ["file", "environment"]:
            for trace, verbose in [(False, False), (False, True), (True, False), (True, True)]:
                with self.subTest(key_source=key_source, trace=trace, verbose=verbose):
                    self.run_fixture(
                        "build_macos_release.sh",
                        key_source=key_source,
                        trace=trace,
                        verbose=verbose,
                    )

    def test_artifact_signer_keeps_file_and_environment_keys_out_of_trace(self):
        for key_source in ["file", "environment"]:
            for trace in [False, True]:
                with self.subTest(key_source=key_source, trace=trace):
                    self.run_fixture("sign_artifacts.sh", key_source=key_source, trace=trace)

    def test_missing_key_fails_without_invoking_signer(self):
        for script_name in ["build_macos_release.sh", "sign_artifacts.sh"]:
            with self.subTest(script_name=script_name):
                self.run_fixture(script_name, key_source="missing", trace=True)


if __name__ == "__main__":
    unittest.main()
