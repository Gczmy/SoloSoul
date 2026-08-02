import { platform, type Platform } from '@tauri-apps/plugin-os';

let cachedPlatform: Platform | null = null;

/**
 * 获取当前 Tauri 运行平台。
 * 结果会缓存，避免重复调用。
 */
export async function getPlatform(): Promise<Platform> {
  if (cachedPlatform) return cachedPlatform;
  cachedPlatform = await platform();
  return cachedPlatform;
}

/**
 * 同步判断是否为移动端（基于缓存）。
 * 若缓存未命中则返回 false，建议在应用初始化时调用一次 getPlatform()。
 */
export function isMobilePlatformSync(): boolean {
  if (!cachedPlatform) return false;
  return cachedPlatform === 'android' || cachedPlatform === 'ios';
}

/**
 * 在应用初始化时预加载平台信息。
 */
export async function initPlatform(): Promise<void> {
  await getPlatform();
}

/**
 * 当前设备是否支持真正的悬停（桌面鼠标）；触屏设备返回 false。
 * 用于门控 JS 模拟 hover（onMouseEnter 驱动的悬浮卡片/内联样式）：
 * Android/iOS WebView 触屏会合成 hover 事件且不自动移除，
 * 若不加门控，点击后 hover 状态会"粘住"造成误解。
 * 注意：CSS :hover 已通过 @media (hover: hover) and (pointer: fine) 统一守卫，
 * 本函数仅用于 JS 逻辑分支（与 CSS 媒体查询同一判定标准）。
 */
export const supportsHover = (): boolean =>
  // matchMedia 缺失（如 jsdom 测试环境）时默认 true：保持既有 hover 行为/测试不受影响；
  // 真实触屏设备（Android/iOS WebView）均实现 matchMedia，会正确返回 false。
  window.matchMedia?.('(hover: hover) and (pointer: fine)').matches ?? true;
