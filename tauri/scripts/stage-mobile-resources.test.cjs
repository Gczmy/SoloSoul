const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createResourceManifest } = require('./stage-mobile-resources.cjs');

function fixture(fn) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'solosoul-resource-manifest-'));
  try {
    fn(root);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}

test('资源版本只由排序后的内容决定，修改时间和路径不影响结果', () =>
  fixture((root) => {
    fs.writeFileSync(path.join(root, 'b.md'), 'b');
    fs.writeFileSync(path.join(root, 'a.md'), 'a');
    const before = createResourceManifest({ docs: root });
    fs.utimesSync(path.join(root, 'a.md'), new Date(0), new Date(0));
    assert.deepEqual(createResourceManifest({ docs: root }), before);
    assert.deepEqual(
      before.files.map((file) => file.path),
      ['docs/a.md', 'docs/b.md'],
    );
    assert.equal(before.files[0].size, 1);
    fs.writeFileSync(path.join(root, 'a.md'), 'changed');
    assert.notEqual(createResourceManifest({ docs: root }).version, before.version);
  }));

test('UTF-8 文件内容和不同根的清单排序稳定', () =>
  fixture((root) => {
    fs.writeFileSync(path.join(root, '中文.md'), '中文');
    const manifest = createResourceManifest({ docs: root });
    assert.equal(manifest.files[0].size, 6);
    assert.equal(manifest.schema, 1);
    assert.match(manifest.version, /^[0-9a-f]{64}$/);
  }));

test('拒绝符号链接，避免把资源目录之外的内容打包', (t) =>
  fixture((root) => {
    fs.writeFileSync(path.join(root, 'a.md'), 'a');
    try {
      fs.symlinkSync(path.join(root, 'a.md'), path.join(root, 'b.md'));
    } catch (error) {
      if (process.platform === 'win32' && error.code === 'EPERM') {
        t.skip('当前 Windows 测试账户没有创建符号链接权限');
        return;
      }
      throw error;
    }
    assert.throws(() => createResourceManifest({ docs: root }), /符号链接/);
  }));

test('缺失源目录不能生成一个伪完整版本', () => {
  assert.throws(
    () =>
      createResourceManifest({ docs: path.join(os.tmpdir(), 'solosoul-missing-resource-source') }),
    /ENOENT/,
  );
});
