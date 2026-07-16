import { create } from 'zustand';

/**
 * 自动锁定暂停计数 —— 模态场景（如密码验证框）打开期间暂停闲置计时。
 *
 * 与 CLI 的 `auto_lock_paused` 语义一致：对话框打开时用户可能长时间无输入，
 * 此时锁定会让已打开的验证框变成孤儿状态。用计数而非布尔值，
 * 以支持多个暂停源嵌套/并存。
 */
interface AutoLockPauseState {
  pauseCount: number;
  pause: () => void;
  resume: () => void;
}

export const useAutoLockPauseStore = create<AutoLockPauseState>((set) => ({
  pauseCount: 0,
  pause: () => set((s) => ({ pauseCount: s.pauseCount + 1 })),
  resume: () => set((s) => ({ pauseCount: Math.max(0, s.pauseCount - 1) })),
}));
