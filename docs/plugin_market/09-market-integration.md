## 10. 与 SoloSoul_plugin_market 的集成

### 10.1 CI/CD 发布流程（替代 build_plugins.sh）

插件不再随 App 打包，而是通过 CI/CD 从 `SoloSoul_plugin_market` 推送到 CDN：

```yaml
# .github/workflows/plugin_release.yml（建议新增）
name: Plugin Release

on:
  push:
    branches: [main]
    paths: ['SoloSoul_plugin_market/**']

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive

      - name: Build plugins
        run: |
          cd SoloSoul_plugin_market
          for dir in plugins/*/; do
            cd "$dir"
            cargo build --target wasm32-wasi --release
            cd -
          done

      - name: Upload to CDN
        run: |
          # 将 wasm + manifest 上传到 S3/R2/CloudFront
          # 更新 registry.json
          aws s3 sync SoloSoul_plugin_market/plugins/ s3://plugins.solosoul.dev/
          aws s3 cp SoloSoul_plugin_market/registry.json s3://plugins.solosoul.dev/registry.json
```

### 10.2 本地插件目录权限

```rust
// Rust 侧初始化时设置目录权限
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(&plugin_dir, perms)?;
}
```
