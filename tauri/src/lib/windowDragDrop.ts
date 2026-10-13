import { logger } from './logger';
import { registerNativeWindowDragDrop } from './nativeWindowDragDrop';

export interface DragDropPayload {
  type: 'enter' | 'over' | 'drop' | 'leave';
  paths?: string[];
  position?: { x: number; y: number };
}

type Handler = (payload: DragDropPayload) => void;
type Subscription = {
  handlers: Set<Handler>;
  pending: boolean;
  unlisten?: () => void;
};

// Module state belongs to this WebView, just like getCurrentWebviewWindow().
let subscription: Subscription | undefined;

function release(state: Subscription) {
  if (subscription === state) subscription = undefined;
  const unlisten = state.unlisten;
  state.unlisten = undefined;
  try {
    unlisten?.();
  } catch (error) {
    logger.warn('[windowDragDrop] Listener cleanup failed:', error);
  }
}

async function register(state: Subscription) {
  try {
    state.unlisten = await registerNativeWindowDragDrop((payload) => {
      // A removed receiver must not run later in this dispatch. New receivers
      // start with the next event, even if mounted during another callback.
      for (const handler of [...state.handlers]) {
        if (!state.handlers.has(handler)) continue;
        try {
          handler(payload);
        } catch (error) {
          logger.warn('[windowDragDrop] Receiver failed:', error);
        }
      }
    });
  } catch (error) {
    // Browser previews retain the existing unavailable-native behavior.
    logger.warn('[windowDragDrop] Listener registration failed:', error);
  } finally {
    state.pending = false;
    if (state.handlers.size === 0) release(state);
  }
}

/** Share one native subscription until the last local receiver is removed. */
export function subscribeWindowDragDrop(handler: Handler): () => void {
  if (!subscription) {
    subscription = { handlers: new Set(), pending: false };
  }
  const state = subscription;
  // Separate ownership even if two consumers pass the same callback.
  const receiver: Handler = (payload) => handler(payload);
  state.handlers.add(receiver);
  if (!state.pending && !state.unlisten) {
    // Retry a failed native registration without discarding existing receivers.
    // Mark pending before starting so simultaneous mounts share this attempt.
    state.pending = true;
    void register(state);
  }
  return () => {
    if (!state.handlers.delete(receiver)) return;
    if (state.handlers.size === 0 && !state.pending) release(state);
    // Keep a pending registration reusable across an immediate remount.
    // If nobody remounts, register's finally releases its eventual handle.
  };
}
