import { invokeCommand } from '@/lib/ipcClient';

let sessionGeneration = 0;
let activeAccountId: string | null = null;
const sessionCleanups = new Set<() => void>();

/** 由认证状态的同步订阅调用；即使后端锁定事件丢失，旧账户请求也立即失效。 */
export function setRequestSession(accountId: string | null): void {
  if (accountId === activeAccountId) return;
  activeAccountId = accountId;
  sessionGeneration += 1;
  for (const clear of sessionCleanups) clear();
}

export function onRequestSessionChange(clear: () => void): void {
  sessionCleanups.add(clear);
}

type Setter<T> = (partial: T | Partial<T> | ((state: T) => T | Partial<T>)) => void;

/** 每个 Store 独立的清理代次，以及同一列表/字段的最新请求序号。 */
export function createSessionRequests() {
  let generation = 0;
  const versions = new Map<string, number>();
  return {
    invalidate(key?: string) {
      if (key !== undefined) versions.set(key, (versions.get(key) ?? 0) + 1);
      else {
        generation += 1;
        versions.clear();
      }
    },
    begin(key?: string, accountId?: string) {
      const session = sessionGeneration;
      const local = generation;
      const matchesAccount = !activeAccountId || !accountId || activeAccountId === accountId;
      const version = key === undefined ? 0 : (versions.get(key) ?? 0) + 1;
      // 旧账户误触发的新调用不能取消当前账户正在读取的同一列表。
      if (key !== undefined && matchesAccount) versions.set(key, version);
      const isCurrent = () =>
        session === sessionGeneration &&
        local === generation &&
        (key === undefined || versions.get(key) === version) &&
        matchesAccount;
      const assertCurrent = () => {
        if (!isCurrent()) throw new Error('Request belongs to an expired session');
      };
      const wait = async <T>(operation: Promise<T>): Promise<T> => {
        try {
          const result = await operation;
          assertCurrent();
          return result;
        } catch (error) {
          assertCurrent();
          throw error;
        }
      };
      const invoke: typeof invokeCommand = async (cmd, args, options) => {
        assertCurrent();
        return wait(invokeCommand(cmd, args, { ...options, requestIsCurrent: isCurrent }));
      };
      return {
        isCurrent,
        assertCurrent,
        invoke,
        guardSet<T>(set: Setter<T>): Setter<T> {
          return (partial) => {
            if (isCurrent()) set(partial);
          };
        },
      };
    },
  };
}
