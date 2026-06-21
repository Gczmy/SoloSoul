import { create } from 'zustand';

interface SidebarHoverState {
  /** Whether the function button area is in hovered/expanded state */
  isHovering: boolean;
  setHovering: (hovering: boolean) => void;
  /** Persisted scroll position for vertical mode (SecondaryActionBar) */
  verticalScrollTop: number;
  setVerticalScrollTop: (scrollTop: number) => void;
  /** Persisted scroll position for horizontal mode (TopFunctionBar) */
  horizontalScrollLeft: number;
  setHorizontalScrollLeft: (scrollLeft: number) => void;
}

/**
 * Global state for the sidebar function button area.
 * Persists across page navigations (which unmount/remount the sidebar),
 * preserving expanded state and scroll positions.
 */
export const useSidebarHoverStore = create<SidebarHoverState>((set) => ({
  isHovering: false,
  setHovering: (hovering) => set({ isHovering: hovering }),
  verticalScrollTop: 0,
  setVerticalScrollTop: (scrollTop) => set({ verticalScrollTop: scrollTop }),
  horizontalScrollLeft: 0,
  setHorizontalScrollLeft: (scrollLeft) => set({ horizontalScrollLeft: scrollLeft }),
}));
