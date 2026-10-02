import { isIOSSync } from './platform';

/** 当前扫描能力的最小门控；模型信息/存储管理不受此限制。 */
export const OCR_UNSUPPORTED_PLATFORM = '__OCR_UNSUPPORTED_PLATFORM__';
export const supportsOcrScanSync = (): boolean => !isIOSSync();
