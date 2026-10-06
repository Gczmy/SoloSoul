/** 只有native-perf初始化注入的固定桥存在时记录；不传账户、密码、响应或错误文本。 */
export type NativeAuthStage =
  | 'login-await-start'
  | 'login-await-ok'
  | 'login-await-error'
  | 'accounts-await-start'
  | 'accounts-await-ok'
  | 'accounts-await-error'
  | 'state-set-start'
  | 'state-set-done'
  | 'finished'
  | 'failed';
interface NativeAuthBridge {
  beginAuthFlow: () => number | null;
  markAuthFlow: (id: number, stage: NativeAuthStage) => void;
  authSnapshot: () => unknown;
}
export function beginNativeAuthTrace(): ((stage: NativeAuthStage) => void) | undefined {
  if (typeof window === 'undefined') return;
  try {
    const bridge = (window as typeof window & { __SOLOSOUL_NATIVE_PERF__?: NativeAuthBridge })
      .__SOLOSOUL_NATIVE_PERF__;
    if (
      !bridge ||
      typeof bridge.beginAuthFlow !== 'function' ||
      typeof bridge.markAuthFlow !== 'function' ||
      typeof bridge.authSnapshot !== 'function'
    )
      return;
    const id = bridge.beginAuthFlow();
    if (!Number.isSafeInteger(id) || id === null || id < 1 || id > 64) return;
    return (stage) => {
      try {
        bridge.markAuthFlow(id, stage);
      } catch {
        /* 观察器失败不能改变业务结果。 */
      }
    };
  } catch {
    return;
  }
}
