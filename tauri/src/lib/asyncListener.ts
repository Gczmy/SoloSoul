import { logger } from './logger';

type Unlisten = () => void;

/**
 * 同步返回 Effect 的清理函数，接管异步返回的事件退订句柄。
 * 即使组件先卸载、订阅后完成，句柄也会被释放；重复清理只释放一次。
 */
export function trackAsyncListener(registration: Promise<Unlisten | null | undefined>): Unlisten {
  let disposed = false;
  let unlisten: Unlisten | null | undefined;
  const release = () => {
    const listener = unlisten;
    unlisten = undefined;
    try {
      listener?.();
    } catch (error) {
      logger.warn('[asyncListener] Listener cleanup failed:', error);
    }
  };
  void registration
    .then((listener) => {
      unlisten = listener;
      if (disposed) release();
    })
    .catch((error) => logger.warn('[asyncListener] Listener registration failed:', error));
  return () => {
    disposed = true;
    release();
  };
}
