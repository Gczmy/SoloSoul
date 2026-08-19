// =============================================================================
// 登录方式持久化（方案 A）
//
// 背景：登录页 `loginMethod` 原本只靠进程内模块缓存，冷启动首次进入登录页时
// 恒为 null → 首帧渲染主密码，等指纹/PIN 可用性探测（2 个异步 IPC）完成后才
// 切换，造成「先显示主密码再跳指纹」闪屏。这里把上次使用的方法按账户持久化到
// localStorage，冷启动首帧即可同步恢复正确方法；后台探测仍会校正过期缓存。
// =============================================================================

/** localStorage key（按账户隔离：{ accountId, method }）。 */
export const LOGIN_METHOD_CACHE_KEY = 'solosoul.loginMethod.v1';

export type LoginMethod = 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password';

const VALID_METHODS: ReadonlySet<string> = new Set([
  'faceId',
  'touchId',
  'windowsHello',
  'pin',
  'password',
]);

interface CachedLoginMethod {
  accountId: string;
  method: LoginMethod;
}

/**
 * 读取指定账户上次使用的登录方式；缓存缺失/账户不匹配/数据损坏时返回 null。
 * 返回 null 时调用方回退为「等待可用性探测」占位（不再闪现主密码）。
 */
export function readCachedLoginMethod(accountId: string): LoginMethod | null {
  if (!accountId) return null;
  try {
    const raw = localStorage.getItem(LOGIN_METHOD_CACHE_KEY);
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return null;
    const cached = parsed as CachedLoginMethod;
    if (cached.accountId !== accountId) return null;
    if (!VALID_METHODS.has(cached.method)) return null;
    return cached.method;
  } catch {
    return null;
  }
}

/**
 * 持久化指定账户的登录方式。accountId 或 method 为空时不写入（不清除旧值）。
 * 存储不可用（隐私模式/配额）时静默忽略。
 */
export function writeCachedLoginMethod(accountId: string, method: LoginMethod | null): void {
  if (!accountId || !method) return;
  try {
    localStorage.setItem(
      LOGIN_METHOD_CACHE_KEY,
      JSON.stringify({ accountId, method } satisfies CachedLoginMethod),
    );
  } catch {
    // 存储不可用时忽略（进程内降级为探测后即时决定）
  }
}
