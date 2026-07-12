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
 * 判断当前是否为移动端（Android 或 iOS）。
 */
export async function isMobilePlatform(): Promise<boolean> {
  const p = await getPlatform();
  return p === 'android' || p === 'ios';
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
