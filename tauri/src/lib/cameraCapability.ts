/**
 * 设备摄像头能力检测（模块级缓存）。
 *
 * 打开软件时预加载一次，供「从其他设备恢复」流程自适应默认 tab：
 * - 支持摄像头 → 默认「扫描二维码」
 * - 不支持摄像头 → 默认「手动输入」
 *
 * 检测使用 `navigator.mediaDevices.enumerateDevices()`，该 API 不会触发
 * 系统权限弹窗（非侵入），仅枚举设备类型；失败时保守视为「不支持」。
 */

export type CameraCapability = 'unknown' | 'supported' | 'unsupported';

let cached: CameraCapability = 'unknown';
let inflight: Promise<CameraCapability> | null = null;

async function detectCameraCapability(): Promise<CameraCapability> {
  try {
    if (typeof navigator === 'undefined' || !navigator.mediaDevices?.enumerateDevices) {
      cached = 'unsupported';
      return cached;
    }
    const devices = await navigator.mediaDevices.enumerateDevices();
    const hasVideoInput = devices.some((d) => d.kind === 'videoinput');
    cached = hasVideoInput ? 'supported' : 'unsupported';
  } catch {
    // 枚举失败（如权限受限/WebView 限制）→ 保守视为不支持，用户仍可走手动模式
    cached = 'unsupported';
  }
  return cached;
}

/** 预加载摄像头能力检测（幂等，应用启动时调用一次）。 */
export function preloadCameraCapability(): Promise<CameraCapability> {
  if (!inflight) {
    inflight = detectCameraCapability();
  }
  return inflight;
}

/** 同步读取已缓存的检测结果（未检测完成时为 'unknown'）。 */
export function getCameraCapability(): CameraCapability {
  return cached;
}
