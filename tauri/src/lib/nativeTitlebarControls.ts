import { invokeCommand } from './ipcClient';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';

type ControlRect = { x: number; y: number; width: number; height: number };
const titlebars = new Map<symbol, { priority: number; regions: ControlRect[] }>();
let lastRegions = '';
let revision = 0;
let queue: Promise<void> = Promise.resolve();
type BackdropLayer = {
  red: number;
  green: number;
  blue: number;
  alpha: number;
  fullWindow: boolean;
  exclusions: Array<ControlRect & { radius: number }>;
};
type BackdropColor = {
  red: number;
  green: number;
  blue: number;
  alpha: number;
  layers?: BackdropLayer[];
};
let backdrop: BackdropColor | undefined;
let maskMutation: MutationObserver | null = null;
const maskObservation = { attributes: true, subtree: true, childList: true };
const suppressed = new Map<HTMLElement, { value: string; priority: string }>();
function restoreMask(element: HTMLElement) {
  const original = suppressed.get(element);
  if (!original) return;
  if (original.value)
    element.style.setProperty('background-color', original.value, original.priority);
  else element.style.removeProperty('background-color');
}

function syncActiveTitlebar() {
  // 覆盖式预览顶栏优先；关闭后立即恢复仍挂载的 AppBar/相册，避免互相覆盖命中区。
  let active: { priority: number; regions: ControlRect[] } | undefined;
  for (const titlebar of titlebars.values()) {
    if (!active || titlebar.priority >= active.priority) {
      active = titlebar;
    }
  }
  const regions = active?.regions ?? [];
  const payload = { regions, ...(backdrop ? { backdrop } : {}) };
  const serialized = JSON.stringify(payload);
  const current = ++revision;
  queue = queue
    .catch(() => {})
    .then(async () => {
      if (current !== revision || serialized === lastRegions) return;
      try {
        await invokeCommand('set_titlebar_controls', payload);
      } catch (error) {
        if (payload.backdrop?.layers?.some((layer) => layer.fullWindow)) {
          const frame = useNativeWindowStore.getState().windowsFrame;
          if (frame)
            useNativeWindowStore.setState({
              windowsFrame: { ...frame, fullWindowBackdrop: false },
            });
        }
        throw error;
      }
      lastRegions = serialized;
    })
    .catch(() => {});
  return queue;
}

function rgba(color: string) {
  const match = /^rgba?\(([^)]+)\)$/.exec(color);
  if (!match) return null;
  const values = match[1]
    .split(/[,\s/]+/)
    .filter(Boolean)
    .map(Number);
  if (values.length < 3 || values.some((n) => !Number.isFinite(n))) return null;
  return [values[0], values[1], values[2], values[3] ?? 1];
}
function foregroundPanels(mask: HTMLElement) {
  const selector = '[data-native-backdrop-panel], [role="dialog"], [data-macos-glass="panel"]';
  let panels = Array.from(mask.querySelectorAll<HTMLElement>(selector));
  if (!panels.length)
    panels = Array.from(mask.parentElement?.children ?? []).filter(
      (el): el is HTMLElement => el instanceof HTMLElement && el.matches(selector),
    );
  return panels.filter((el) => !panels.some((parent) => parent !== el && parent.contains(el)));
}
function maskOrder(element: HTMLElement) {
  for (let current: HTMLElement | null = element; current; current = current.parentElement) {
    const value = Number(getComputedStyle(current).zIndex);
    if (value > 0) return value;
  }
  return 0;
}
/** Native-controlled masks are transparent in the DOM. The native bitmap owns all their dimming. */
function syncBackdrops() {
  const observing = maskMutation;
  observing?.disconnect();
  try {
    return collectBackdrops();
  } finally {
    if (observing && observing === maskMutation)
      observing.observe(document.documentElement, maskObservation);
  }
}
function collectBackdrops() {
  // Restore only our own override before measuring a changed theme; these writes never reach paint.
  for (const [element, original] of suppressed) {
    if (element.style.backgroundColor !== 'transparent') {
      original.value = element.style.getPropertyValue('background-color');
      original.priority = element.style.getPropertyPriority('background-color');
    }
    restoreMask(element);
  }
  const masks = useNativeWindowStore.getState().windowsFrame
    ? Array.from(document.querySelectorAll<HTMLElement>('[data-macos-glass-backdrop]'))
        .filter((el) => {
          const r = el.getBoundingClientRect();
          const style = getComputedStyle(el);
          return (
            r.x <= 0 &&
            r.y <= 0 &&
            r.right >= window.innerWidth &&
            r.bottom >= window.innerHeight &&
            style.display !== 'none' &&
            style.visibility !== 'hidden' &&
            el.checkVisibility?.({ checkOpacity: true, checkVisibilityCSS: true }) !== false
          );
        })
        .sort((a, b) => maskOrder(a) - maskOrder(b))
    : [];
  const fullWindow =
    useNativeWindowStore.getState().windowsFrame?.fullWindowBackdrop === true &&
    masks.some((mask) => mask.dataset.nativeWindowBackdrop === 'true' || suppressed.has(mask));
  for (const [element] of suppressed) {
    if (!fullWindow || !masks.includes(element)) {
      restoreMask(element);
      suppressed.delete(element);
    }
  }
  const active = masks.filter((mask) => mask.dataset.nativeBackdropClosing !== 'true');
  const layers: BackdropLayer[] = [];
  let alpha = 0;
  let rgb = [0, 0, 0];
  for (const [index, el] of active.entries()) {
    const style = getComputedStyle(el);
    const controlled = el.dataset.nativeWindowBackdrop === 'true';
    const color = controlled
      ? (rgba(style.getPropertyValue('--bg-overlay').trim()) ?? rgba(style.outlineColor))
      : rgba(style.backgroundColor);
    if (!color) continue;
    const nextAlpha = color[3] * Number(style.opacity || 1);
    if (nextAlpha <= 0) continue;
    const combined = nextAlpha + alpha * (1 - nextAlpha);
    rgb = rgb.map(
      (channel, i) => (color[i] * nextAlpha + channel * alpha * (1 - nextAlpha)) / combined,
    );
    alpha = combined;
    const panels = active.slice(index).flatMap(foregroundPanels);
    // Field menus and toasts above a card must remain clear even outside its rectangle.
    const floating = Array.from(
      document.querySelectorAll<HTMLElement>(
        '[data-macos-glass="panel"], [data-preview-window], [role="dialog"], [data-macos-glass="popover"], [data-macos-glass="tooltip"], [data-macos-glass="toast"], [role="menu"], [role="listbox"]',
      ),
    ).filter((panel) => {
      const style = getComputedStyle(panel);
      return (
        style.display !== 'none' &&
        style.visibility !== 'hidden' &&
        panel.checkVisibility?.({ checkOpacity: true, checkVisibilityCSS: true }) !== false &&
        (panels.some((card) => card.contains(panel)) || maskOrder(panel) > maskOrder(el))
      );
    });
    const exclusions = [...panels, ...floating]
      .map((panel) => {
        const r = panel.getBoundingClientRect();
        return {
          x: r.x,
          y: r.y,
          width: r.width,
          height: r.height,
          radius: parseFloat(getComputedStyle(panel).borderTopLeftRadius) || 0,
        };
      })
      .filter((r) => r.width > 0 && r.height > 0);
    if (fullWindow && !controlled) {
      if (!suppressed.has(el))
        suppressed.set(el, {
          value: el.style.getPropertyValue('background-color'),
          priority: el.style.getPropertyPriority('background-color'),
        });
      el.style.setProperty('background-color', 'transparent', 'important');
    }
    layers.push({
      red: Math.round(color[0]),
      green: Math.round(color[1]),
      blue: Math.round(color[2]),
      alpha: Math.round(nextAlpha * 255),
      fullWindow,
      exclusions,
    });
  }
  const next: BackdropColor = {
    red: Math.round(rgb[0]),
    green: Math.round(rgb[1]),
    blue: Math.round(rgb[2]),
    alpha: Math.round(alpha * 255),
    ...(fullWindow ? { layers } : {}),
  };
  if (!backdrop && next.alpha === 0) return queue;
  if (JSON.stringify(backdrop) === JSON.stringify(next)) return queue;
  backdrop = next;
  return syncActiveTitlebar();
}
/** Release this mask before unmounting its card, so its cutout cannot expose the page during close. */
export async function releaseNativeWindowBackdrop(element: HTMLElement) {
  element.dataset.nativeBackdropClosing = 'true';
  await syncBackdrops();
  // A newer geometry/stack request can supersede the first request. Wait for the latest acknowledgement.
  for (;;) {
    const pending = queue;
    await pending;
    if (pending === queue) return;
  }
}
export function observeNativeCaptionBackdrops() {
  let stopped = false;
  const sizes = new ResizeObserver(() => {
    if (!stopped) void syncBackdrops();
  });
  const sync = () => {
    if (stopped) return;
    sizes.disconnect();
    document.querySelectorAll<HTMLElement>('[data-macos-glass-backdrop]').forEach((mask) => {
      sizes.observe(mask);
      foregroundPanels(mask).forEach((panel) => sizes.observe(panel));
    });
    void syncBackdrops();
  };
  // Mutation callbacks run before paint; no additional RAF or duplicate DOM dimming.
  const mutation = new MutationObserver(sync);
  maskMutation = mutation;
  mutation.observe(document.documentElement, maskObservation);
  window.addEventListener('resize', sync);
  sync();
  return () => {
    stopped = true;
    mutation.disconnect();
    if (maskMutation === mutation) maskMutation = null;
    for (const element of suppressed.keys()) restoreMask(element);
    suppressed.clear();
    sizes.disconnect();
    window.removeEventListener('resize', sync);
    if (backdrop && backdrop.alpha > 0) {
      backdrop = { red: 0, green: 0, blue: 0, alpha: 0 };
      syncActiveTitlebar();
    }
  };
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
