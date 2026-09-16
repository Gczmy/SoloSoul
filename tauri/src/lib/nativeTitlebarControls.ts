import { invokeCommand } from './ipcClient';

type ControlRect = { x: number; y: number; width: number; height: number };
const titlebars = new Map<symbol, { priority: number; regions: ControlRect[] }>();
let lastRegions = '';

function syncActiveTitlebar() {
  // 覆盖式预览顶栏优先；关闭后立即恢复仍挂载的 AppBar/相册，避免互相覆盖命中区。
  let active: { priority: number; regions: ControlRect[] } | undefined;
  for (const titlebar of titlebars.values()) {
    if (!active || titlebar.priority >= active.priority) active = titlebar;
  }
  const regions = active?.regions ?? [];
  const serialized = JSON.stringify(regions);
  if (serialized === lastRegions) return;
  lastRegions = serialized;
  void invokeCommand('set_titlebar_controls', { regions }).catch(() => {});
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
    const regions = Array.from(header.querySelectorAll<HTMLElement>(selector))
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0)
      .map(({ x, y, width, height }) => ({ x, y, width, height }));
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
