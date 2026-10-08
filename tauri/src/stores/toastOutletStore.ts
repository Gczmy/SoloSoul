import { create } from 'zustand';

interface Outlet {
  node: HTMLElement;
  priority: number;
}

function topOutlet(outlets: Outlet[]) {
  // 同层级最后挂载的浮层优先；关闭后自动交回仍挂载的宿主。
  return (
    outlets.reduce<Outlet | null>(
      (top, outlet) => (!top || outlet.priority >= top.priority ? outlet : top),
      null,
    )?.node ?? null
  );
}

/** 仅保存当前挂载的通知槽，不持久化 DOM，也不接管 Toast 的数据或计时。 */
export const useToastOutletStore = create<{
  outlets: Outlet[];
  target: HTMLElement | null;
  register: (node: HTMLElement, priority: number) => () => void;
}>((set) => ({
  outlets: [],
  target: null,
  register: (node, priority) => {
    const outlet = { node, priority };
    set((state) => {
      const outlets = [...state.outlets, outlet];
      return { outlets, target: topOutlet(outlets) };
    });
    return () =>
      set((state) => {
        const outlets = state.outlets.filter((entry) => entry !== outlet);
        return { outlets, target: topOutlet(outlets) };
      });
  },
}));
