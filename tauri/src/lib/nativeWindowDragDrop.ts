import { PhysicalPosition } from '@tauri-apps/api/dpi';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { DragDropPayload } from './windowDragDrop';
import { logger } from './logger';

type NativePayload = { paths?: string[]; position: { x: number; y: number } };

async function unregisterAll(unlisten: UnlistenFn[]) {
  const results = await Promise.allSettled(unlisten.splice(0).map(async (stop) => stop()));
  for (const result of results) {
    if (result.status === 'rejected') {
      logger.warn('[windowDragDrop] Native listener cleanup failed:', result.reason);
    }
  }
}

/** Same four window events and physical positions as Tauri's onDragDropEvent,
 * with rollback if any registration fails before the aggregate handle exists.
 */
export async function registerNativeWindowDragDrop(
  handler: (payload: DragDropPayload) => void,
): Promise<UnlistenFn> {
  const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
  const window = getCurrentWebviewWindow();
  const unlisten: UnlistenFn[] = [];
  let active = true;
  try {
    for (const type of ['enter', 'over', 'drop', 'leave'] as const) {
      unlisten.push(
        await window.listen<NativePayload>(`tauri://drag-${type}`, (event) => {
          if (!active) return;
          if (type === 'leave') {
            handler({ type });
          } else {
            const position = new PhysicalPosition(event.payload.position);
            handler(
              type === 'over' ? { type, position } : { type, position, paths: event.payload.paths },
            );
          }
        }),
      );
    }
  } catch (error) {
    active = false;
    await unregisterAll(unlisten);
    throw error;
  }
  return () => {
    if (!active) return;
    active = false;
    void unregisterAll(unlisten);
  };
}
