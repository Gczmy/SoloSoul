import { invokeCommand } from './ipcClient';

/** 将顶部真实控件交给 WebView 点击，原生拖拽带只处理剩余空白。 */
export function observeTitlebarControls(header: HTMLElement) {
  let frame = 0;
  let last = '';
  const sync = () => {
    const regions = Array.from(header.querySelectorAll<HTMLElement>('[data-titlebar-control]'))
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0)
      .map(({ x, y, width, height }) => ({ x, y, width, height }));
    const serialized = JSON.stringify(regions);
    if (serialized === last) return;
    last = serialized;
    void invokeCommand('set_titlebar_controls', { regions }).catch(() => {});
  };
  const schedule = () => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(sync);
  };
  const resize = new ResizeObserver(schedule);
  const observeSizes = () => {
    resize.disconnect();
    resize.observe(header);
    header
      .querySelectorAll('[data-titlebar-control]')
      .forEach((element) => resize.observe(element));
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
    cancelAnimationFrame(frame);
    resize.disconnect();
    mutation.disconnect();
    window.removeEventListener('resize', schedule);
    void invokeCommand('set_titlebar_controls', { regions: [] }).catch(() => {});
  };
}
