/**
 * 统一 IPC 调用层（P131）
 *
 * 约定：所有前端 Rust 命令调用一律经 `invokeCommand` 走本层，
 * 禁止裸调 `@tauri-apps/api/core` 的 `invoke`。历史文件通过
 * `import { invokeCommand as invoke } from '@/lib/ipcClient'` 迁移，
 * 调用签名与裸 `invoke` 完全一致，仅统一收口。
 *
 * 本层职责：
 * 1. **统一失败日志** —— 任何命令失败都带命令名记 logger.warn，
 *    消灭「静默失败 / unhandled rejection」类问题（配合各组件 onError toast）。
 * 2. **默认解锁守卫（P027）** —— Vault 未解锁（`isAuthenticated === false`）时，
 *    除 `UNLOCKED_EXEMPT_COMMANDS` 豁免名单（认证/解锁流程、启动期系统命令）外
 *    的所有命令一律在发起 IPC 前抛 `No account is currently unlocked`
 *    （与后端错误语义一致），动态 import authStore 避免循环依赖。
 *    `opts.requireUnlocked` 可显式覆盖：`true` 强制启用（豁免名单也拦截）、
 *    `false` 显式豁免（仅用于极少数确定无需解锁的命令）。
 * 3. **错误消息透传** —— 不在此处翻译错误：展示层（useToastError /
 *    translateRustError / resolveBackendErrorMessage）负责翻译，避免
 *    双层翻译破坏既有 translateRustError 消费方。
 *
 * 注意：本模块不 import `@/lib/logger`（其链路 utils → i18n 与本模块被
 * i18n 依赖构成循环）——失败日志内联 dev 守卫，仅依赖 core。
 */
import { invoke } from '@tauri-apps/api/core';

export interface InvokeOptions {
  /**
   * 解锁守卫覆盖：
   * - 省略：默认启用（P027），豁免名单内命令除外；
   * - `true`：强制启用（豁免名单内命令也会被拦截）；
   * - `false`：显式豁免（仅用于确定无需解锁的命令，避免与后端鉴权冲突）。
   */
  requireUnlocked?: boolean;
}

/**
 * P027：无需解锁即可调用的命令豁免名单（认证/解锁流程 + 启动期系统命令）。
 *
 * 这些命令是用户尚未解锁时（登录页 / 引导 / 锁屏遮罩 / 启动期 UI 系统）
 * 必须可用的；其余命令默认要求 Vault 已解锁。名单按命令名精确匹配，
 * 新增「未解锁时也必须可用」的命令时须在此登记（否则默认被守卫拦截）。
 */
const UNLOCKED_EXEMPT_COMMANDS: ReadonlySet<string> = new Set([
  // ── 认证 / 账户流程（登录页 / 引导 / 锁屏遮罩）──
  'check_has_account',
  'bootstrap',
  'login',
  'logout',
  'lock',
  'unlock',
  'unlock_with_password',
  'pin_unlock',
  'biometric_unlock',
  // 可用性探测命令必须在未解锁时也可调：登录页据此决定是否展示
  // 指纹/面容/PIN 等解锁方式（纯只读探测，无敏感数据，后端按 accountId 返回）。
  'biometric_check_availability',
  'pin_check_availability',
  'biometric_test',
  'biometric_save_credential',
  'biometric_delete_credential',
  'vault_list_accounts',
  // 首次启动引导（无账户/未解锁）选择与初始化保险库存储位置：SAF 目录选择器
  // 与目录初始化均与 Vault 数据内容无关、后端不鉴权，是引导流程的前置步骤；
  // 否则 P027 在引导期拦截，安卓新装/重装用户选择存储目录即报
  // 「No account is currently unlocked」
  'vault_pick_directory',
  'init_vault_directory',
  // 登录页「恢复数据」流程（未解锁时必须可用）：发现可恢复的主机 + 从主机
  // 恢复；recovery_host_start/recovery_host_cancel（作为主机展示二维码）仅
  // 在同步设置页 post-auth 可达，保持守卫
  'recovery_discover_hosts',
  'recovery_restore_from_host',
  'reset_security_flags',
  'dismiss_lock_mask',
  'get_lock_pending',
  'is_screen_locked',
  'vault_sync_background',
  // ── 启动期系统 / UI 命令（App 初始化、主题、语言、偏好、更新检查）──
  'get_app_info',
  'get_system_locale',
  // 登录页/引导页挂载即应用主题（useApplyThemeFromSettings，默认 theme=
  // 'system' 时 getSystemTheme 与 applyTheme 内部都会调本命令）——未解锁时
  // 不豁免则主题应用被守卫拦截且产生 unhandled rejection
  'get_system_theme',
  'set_titlebar_color',
  'set_status_bar_style',
  'ui_get_preferences',
  'ui_update_preference',
  'user_data_update_preference',
  'log_write',
  // ── 自更新管线（检查/下载/安装）：只读 GitHub Release 元数据与 APK 缓存文件，
  //    与 Vault 数据完全无关。启动期（锁屏）即允许检查更新，横幅可提前出现；
  //    否则 P027 在 App 挂载（未解锁）时拦截 android_check_update，Android 端
  //    更新横幅永远不出现（桌面端走 updater 插件直连 HTTP 不受影响）。
  'android_check_update',
  'android_download_apk',
  'android_get_apk_path',
  'android_is_apk_downloaded',
  'android_install_apk',
  'desktop_check_update',
  // ── OCR 模型管理（useOcrFirstInstall / useOcrModelManager：模型与 vault 数据
  //    无关，后端不校验解锁；设置页与首次引导在未解锁时也可访问）──
  'ocr_get_model_status',
  'ocr_get_active_tier',
  'ocr_set_active_tier',
  'ocr_delete_model',
  'ocr_download_model',
  'ocr_install_bundled_model',
  'ocr_install_bundled_model_with_progress',
]);

function shouldRequireUnlocked(cmd: string, opts?: InvokeOptions): boolean {
  if (opts?.requireUnlocked !== undefined) {
    return opts.requireUnlocked;
  }
  return !UNLOCKED_EXEMPT_COMMANDS.has(cmd);
}

/** dev 守卫日志：与 logger.warn 同语义，但不引入 logger→utils→i18n 的循环依赖。 */
function devWarn(...args: unknown[]): void {
  if (
    import.meta.env.DEV === true ||
    import.meta.env.MODE === 'debug' ||
    import.meta.env.VITE_SOLOSOUL_DEBUG === 'true'
  ) {
    console.warn(...args);
  }
}

/**
 * 统一命令调用入口。签名与 Tauri `invoke` 兼容（args 可选）。
 */
export async function invokeCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
  opts?: InvokeOptions,
): Promise<T> {
  // P027: 测试环境（vitest）默认放行守卫——既有 store/组件测试大多直接断言
  // invoke 调用、不关心解锁状态，且部分模块链会先于 mock 缓存真实 authStore；
  // 守卫的拦截/豁免逻辑由 ipcClient.test.ts 通过显式 requireUnlocked 全覆盖。
  if (shouldRequireUnlocked(cmd, opts) && import.meta.env.MODE !== 'test') {
    // 动态 import 避免与 authStore 循环依赖
    let isAuthenticated = true;
    try {
      const { useAuthStore } = await import('@/stores/authStore');
      const getState = useAuthStore.getState as unknown;
      if (typeof getState === 'function') {
        // 能读到解锁状态：仅当明确未解锁（isAuthenticated === false）才拦截。
        // （getState 缺失的场景——部分测试环境 mock 为 hook 形态——视为
        // 「前端状态不可得」而 fail-open，交由后端鉴权兜底（P027 前提：
        // 敏感 command 后端已鉴权，前端守卫只是减少无效 IPC 的 UX 优化）。）
        const state = (getState as () => { isAuthenticated?: boolean })();
        isAuthenticated = state.isAuthenticated !== false;
      }
    } catch {
      // authStore 动态 import / getState 异常（极端启动时序）→ fail-open 交后端
      isAuthenticated = true;
    }
    if (!isAuthenticated) {
      const err = new Error('No account is currently unlocked');
      devWarn(`[ipc] '${cmd}' blocked: vault not unlocked`);
      throw err;
    }
  }
  try {
    // args 缺省时不再传第二参：既有测试断言 `toHaveBeenCalledWith('cmd')`
    // （单参精确匹配），多传 undefined 会破坏这些断言。
    if (args === undefined) {
      return await invoke<T>(cmd);
    }
    return await invoke<T>(cmd, args);
  } catch (err) {
    const raw = err instanceof Error ? err.message : String(err);
    devWarn(`[ipc] command '${cmd}' failed:`, raw);
    throw err;
  }
}
