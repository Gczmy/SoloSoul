const { spawnSync } = require('node:child_process');

// 仅在解释器不存在时尝试下一个入口，检查失败必须原样传回，避免掩盖错误。
const candidates =
  process.platform === 'win32'
    ? [
        ['python', []],
        ['py', ['-3']],
        ['python3', []],
      ]
    : [
        ['python3', []],
        ['python', []],
      ];
const args = process.argv.slice(2);
if (args.length === 0) {
  console.error('Usage: node scripts/run-python-check.cjs <script> [args...]');
  process.exit(1);
}
for (const [command, prefix] of candidates) {
  const result = spawnSync(command, [...prefix, '-X', 'utf8', ...args], {
    stdio: 'inherit',
    windowsHide: true,
  });
  if (result.error?.code === 'ENOENT') continue;
  if (result.error) console.error(result.error.message);
  process.exit(result.status ?? 1);
}
console.error('Python 3 was not found. Install Python 3 and add it to PATH.');
process.exit(1);
