/**
 * P029: 两套后端错误库合并——Rust 静态错误映射已并入 `./backendError`（单一入口）。
 * 本文件保留为兼容 re-export，旧 import 路径（`@/lib/rustErrors`）无需改动。
 * 新代码请从 `@/lib/backendError` 导入 `translateRustError` / `resolveBackendErrorMessage`。
 */
export { translateRustError } from './backendError';
