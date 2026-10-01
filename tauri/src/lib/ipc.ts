export type {
  SyncConflictDto as SyncConflict,
  ConflictHlc as SyncConflictHlc,
  ConflictSummary as SyncConflictSummary,
  ConflictDetail as SyncConflictDetail,
  TableResult as SyncTableResult,
} from './generated/ipcContracts';
export type { SyncResult, SyncConflictStrategy } from './syncViewModel';

export interface AccountInfo {
  id: string;
  name: string;
  /** P022: salt/verifyHash 已从后端 DTO 移除（前端零消费，扩大攻击面） */
  passwordHint?: string;
  createdAt?: string;
  /** 该账户是否曾在卸载前启用过生物识别（指纹/人脸），引导用户重新设置。 */
  hasBiometricHistory?: boolean;
  /** 该账户是否曾在卸载前启用过 PIN 码解锁。 */
  hasPinHistory?: boolean;
}

export interface OcrBox {
  text: string;
  confidence: number;
  points: [number, number][];
}

export interface OcrResult {
  text: string;
  confidence: number;
  boxes: OcrBox[];
}

export interface MrzResult {
  documentType: string;
  documentTypeSub: string;
  issuingCountry: string;
  documentNumber: string;
  checkDigitDocumentNumber: string;
  nationality: string;
  dateOfBirth: string;
  checkDigitDateOfBirth: string;
  sex: string;
  expiryDate: string;
  checkDigitExpiry: string;
  optionalData: string;
  compositeCheckDigit: string;
  rawLines: string[];
  confidence: number;
  checksumValid: boolean;
}

export interface OcrTierInfo {
  tier: string;
  name: string;
  description: string;
}

export interface OcrModelStatus {
  tier: string;
  installed: boolean;
  bundled: boolean;
  /** P133: 系统内置引擎（macOS Vision）——前端据此隐藏安装/下载/删除等模型管理操作。 */
  builtin?: boolean;
}
