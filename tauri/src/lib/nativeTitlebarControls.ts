import { invokeCommand } from './ipcClient';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';

type ControlRect = { x: number; y: number; width: number; height: number };
const titlebars = new Map<symbol, { priority: number; regions: ControlRect[] }>();
let lastRegions = '';
let revision = 0;
let queue: Promise<void> = Promise.resolve();

function syncActiveTitlebar() {
  // 覆盖式预览顶栏优先；关闭后立即恢复仍挂载的 AppBar/相册，避免互相覆盖命中区。
  let active: { priority: number; regions: ControlRect[] } | undefined;
  for (const titlebar of titlebars.values()) {
    if (!active || titlebar.priority >= active.priority) {
      active = titlebar;
    }
  }
  const regions = active?.regions ?? [];
  const serialized = JSON.stringify(regions);
  const current = ++revision;
  queue = queue
    .catch(() => {})
    .then(async () => {
      if (current !== revision || serialized === lastRegions) return;
      await invokeCommand('set_titlebar_controls', { regions });
      lastRegions = serialized;
    })
    .catch(() => {});
}

/** 将顶部真实控件交给 WebView 点击，原生拖拽带只处理剩余空白。 */
export function observeTitlebarControls(
  header: HTMLElement,
  { priority = 0, selector = '[data-titlebar-control]' } = {},
) {
  const token = Symbol('titlebar');
  titlebars.set(token, { priority, regions: [] });
  let frame = 0;
  let stopped = false;
  const sync = () => {
    if (stopped) return;
    const windows = Boolean(useNativeWindowStore.getState().windowsFrame);
    const regions = Array.from(header.querySelectorAll<HTMLElement>(selector))
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0)
      .map(({ x, y, width, height }) =>
        windows
          ? {
              x: Math.max(0, x),
              y: Math.max(0, y),
              width: Math.max(0, Math.min(window.innerWidth, x + width) - Math.max(0, x)),
              height: Math.max(0, Math.min(52, y + height) - Math.max(0, y)),
            }
          : { x, y, width, height },
      )
      .filter((r) => r.width > 0 && r.height > 0);
    titlebars.set(token, { priority, regions });
    syncActiveTitlebar();
  };
  const schedule = () => {
    if (stopped) return;
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(sync);
  };
  const resize = new ResizeObserver(schedule);
  const observeSizes = () => {
    resize.disconnect();
    resize.observe(header);
    header.querySelectorAll(selector).forEach((element) => resize.observe(element));
  };
  observeSizes();
  const mutation = new MutationObserver(() => {
    observeSizes();
    schedule();
  });
  mutation.observe(header, {
    attributes: true,
    childList: true,
    subtree: true,
    characterData: true,
  });
  window.addEventListener('resize', schedule);
  sync();
  return () => {
    stopped = true;
    cancelAnimationFrame(frame);
    resize.disconnect();
    mutation.disconnect();
    window.removeEventListener('resize', schedule);
    titlebars.delete(token);
    syncActiveTitlebar();
  };
}
