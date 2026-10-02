import {
  getPlatformCapabilitiesSync,
  usePlatformCapabilities,
  type PlatformCapability,
} from './platformCapabilities';

export const OCR_UNSUPPORTED_PLATFORM = '__OCR_UNSUPPORTED_PLATFORM__';
export const OCR_CAPABILITY_UNAVAILABLE = '__OCR_CAPABILITY_UNAVAILABLE__';

/** 模型信息/存储管理独立保留；仅扫描动作依赖后端能力。 */
export const supportsOcrScanSync = (): boolean =>
  getPlatformCapabilitiesSync().ocr.status === 'supported';
export const useOcrScanCapability = (): PlatformCapability => usePlatformCapabilities().ocr;
export const ocrCapabilityError = (): string =>
  getPlatformCapabilitiesSync().ocr.status === 'unsupported'
    ? OCR_UNSUPPORTED_PLATFORM
    : OCR_CAPABILITY_UNAVAILABLE;
export const ocrCapabilityMessageKey = (capability: PlatformCapability): string =>
  capability.reason === 'ios_ocr_not_implemented'
    ? 'ocr:ios_ocr_unsupported'
    : capability.status === 'unsupported'
      ? 'ocr:scan_platform_unsupported'
      : 'ocr:scan_capability_unavailable';
