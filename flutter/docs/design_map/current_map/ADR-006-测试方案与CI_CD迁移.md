# ADR-006: 测试方案与 CI/CD 迁移

> **状态**: 已采纳 ✅  
> **决策日期**: 2026-06-04  
> **决策人**: SoloSoul 架构组  
> **影响范围**: 测试策略、CI/CD 流水线、质量保障

---

## 背景

当前测试体系：
- **Rust**: `flutter/native/` — `cargo test`
- **Go**: `go test -tags "rust cgo" ./...`
- **Dart**: `flutter test test/unit/` + `flutter test test/widget/` + `flutter test integration_test/`

Tauri 迁移后，技术栈变为 Rust + TypeScript，测试体系需要全面更新。

---

## 测试金字塔

```
                    ▲
                   /  \
                  / E2E \        Playwright（5%）
                 /________\
                /          \
               / Integration \   Rust 集成测试 + 前端组件测试（15%）
              /______________\
             /                \
            /     Unit Tests    \  Rust 单元测试 + Vitest（80%）
           /______________________\
```

---

## 测试分层

### 第一层：Rust 单元测试

**范围**: `crates/` 和 `src-tauri/src/core/`

**工具**: `cargo test`（内置）

**测试内容**:
- 密码学（Argon2id 派生 + AES-GCM 往返）
- Vault 生命周期（init/unlock/lock/changePassword）
- Profile 模型验证
- CRDT 合并
- MRZ 解析
- 字段验证正则

**代码位置**:
```
crates/solosoul-crypto/src/
  ├── kdf.rs
  ├── kdf_test.rs          # #[cfg(test)] mod tests { ... }
  ├── cipher.rs
  └── cipher_test.rs

src-tauri/src/core/
  ├── profile/
  │   ├── model.rs
  │   └── model_test.rs
  └── utils/
      ├── validator.rs
      └── validator_test.rs
```

**CI 命令**:
```bash
cargo test --workspace --verbose
cargo test --workspace --release  # 发布模式测试（检测优化问题）
```

---

### 第二层：前端单元测试

**范围**: `src/` 中的组件和 hooks

**工具**: **Vitest** + React Testing Library

**不选 Jest 的原因**:
- Vite 原生集成 Vitest，零配置
- 速度比 Jest 快
- 支持 ESM 原生

**安装**:
```bash
npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

**配置** (`vitest.config.ts`):
```typescript
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
  },
});
```

**测试示例**:
```typescript
// src/components/ui/Button.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { Button } from './Button';
import { describe, it, expect, vi } from 'vitest';

describe('Button', () => {
  it('renders with text', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText('Click me')).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const handleClick = vi.fn();
    render(<Button onClick={handleClick}>Click</Button>);
    fireEvent.click(screen.getByText('Click'));
    expect(handleClick).toHaveBeenCalledTimes(1);
  });
});
```

---

### 第三层：集成测试

**范围**: Rust services + 数据库

**工具**: `cargo test` + `tempfile` + `rusqlite`（内存数据库）

**测试示例**:
```rust
#[tokio::test]
async fn test_vault_unlock_lock_cycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = VaultManager::new(temp_dir.path()).unwrap();
    
    // Initialize
    manager.initialize("test_account", "password123").await.unwrap();
    assert_eq!(manager.state().await, VaultState::Unlocked);
    
    // Lock
    manager.lock().await.unwrap();
    assert_eq!(manager.state().await, VaultState::Locked);
    
    // Unlock
    manager.unlock("test_account", "password123").await.unwrap();
    assert_eq!(manager.state().await, VaultState::Unlocked);
    
    // Wrong password
    let result = manager.unlock("test_account", "wrong").await;
    assert!(result.is_err());
}
```

---

### 第四层：端到端测试（E2E）

**范围**: 完整用户流程

**工具**: **Playwright**

**不选 Cypress 的原因**:
- Playwright 支持多浏览器（Chrome, Firefox, Safari）
- Playwright 支持 Tauri 应用测试（通过 `tauri-driver`）
- 更快的执行速度
- 更好的并行支持

**安装**:
```bash
npm install -D @playwright/test
npx playwright install
```

**配置** (`playwright.config.ts`):
```typescript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
});
```

**测试示例**:
```typescript
// tests/e2e/auth.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Authentication', () => {
  test('bootstrap and login flow', async ({ page }) => {
    // 首次使用：引导页面
    await page.goto('/bootstrap');
    await expect(page.locator('h1')).toContainText('欢迎使用 SoloSoul');
    
    // 创建账户
    await page.fill('[name="accountName"]', 'Test Account');
    await page.fill('[name="password"]', 'SecurePassword123!');
    await page.fill('[name="confirmPassword"]', 'SecurePassword123!');
    await page.click('button[type="submit"]');
    
    // 验证跳转到首页
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-testid="welcome-message"]')).toBeVisible();
  });

  test('vault auto-lock', async ({ page }) => {
    // 登录
    await page.goto('/login');
    await page.fill('[name="password"]', 'SecurePassword123!');
    await page.click('button[type="submit"]');
    
    // 等待自动锁定（测试中设置较短时间）
    await page.waitForTimeout(5000);
    
    // 验证跳转到登录页
    await expect(page).toHaveURL('/login');
  });
});
```

**Tauri 应用测试**（通过 `tauri-driver`）:
```bash
# 启动 tauri-driver
tauri-driver --port 4444

# Playwright 连接 tauri-driver
```

---

## CI/CD 迁移

### 当前 CI（.github/workflows/）

```yaml
# ci_cd.yml（当前 Flutter）
jobs:
  rust-test:        # cargo test (flutter/native/)
  dart-unit-test:   # flutter test test/unit/
  widget-test:      # flutter test test/widget/
  integration-test: # flutter test integration_test/
  release:          # flutter build macos + DMG
```

### 新 CI（Tauri）

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:

jobs:
  # 1. Rust 检查 + 测试
  rust-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Check formatting
        run: cargo fmt --check
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Run tests
        run: cargo test --workspace --verbose

  # 2. 前端检查 + 测试
  frontend-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - name: Install dependencies
        run: npm ci
      - name: Type check
        run: npx tsc --noEmit
      - name: Lint
        run: npm run lint
      - name: Unit tests
        run: npx vitest run

  # 3. E2E 测试（macOS）
  e2e-test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - name: Install dependencies
        run: npm ci
      - name: Install Playwright
        run: npx playwright install
      - name: Build Tauri
        run: npm run tauri build -- --debug
      - name: Run E2E tests
        run: npx playwright test

  # 4. 构建（多平台）
  build:
    strategy:
      matrix:
        platform: [macos-latest, ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - name: Install dependencies
        run: npm ci
      - name: Build
        run: npm run tauri build
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: solo-soul-${{ matrix.platform }}
          path: src-tauri/target/release/bundle/

  # 5. 发布（仅 main push）
  release:
    if: github.ref == 'refs/heads/main'
    needs: [rust-check, frontend-check, e2e-test, build]
    runs-on: macos-latest
    steps:
      - name: Create Draft Release
        uses: softprops/action-gh-release@v2
        with:
          draft: true
          files: |
            src-tauri/target/release/bundle/**/*.dmg
            src-tauri/target/release/bundle/**/*.AppImage
            src-tauri/target/release/bundle/**/*.msi
```

---

## 测试覆盖率

### Rust 覆盖率

**工具**: `cargo-tarpaulin`

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html --out Lcov
```

### 前端覆盖率

**工具**: Vitest 内置 + `@vitest/coverage-v8`

```bash
npm install -D @vitest/coverage-v8
npx vitest run --coverage
```

### 目标覆盖率

| 层 | 目标覆盖率 | 说明 |
|----|-----------|------|
| Rust 单元测试 | 80%+ | 密码学、Vault、核心业务逻辑 |
| 前端单元测试 | 60%+ | 组件、Hooks、工具函数 |
| E2E 测试 | 核心流程覆盖 | 登录、CRUD、设置、备份 |

---

## 相关文档

- `L8_测试与质量保障层.md` — 当前测试体系
- `tauri_refactor/测试方案.md` — 具体测试迁移实施

---

*文档版本：v1.0*  
*创建日期：2026-06-04*
