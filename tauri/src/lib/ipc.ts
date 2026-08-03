export interface SyncConflict {
  table: string;
  id: string;
  local_hlc: {
    wall_time_ms: number;
    counter: number;
    node_id: number[];
  };
  remote_hlc: {
    wall_time_ms: number;
    counter: number;
    node_id: number[];
  };
  winner: string;
}

export interface SyncConflictHlc {
  wall_time_ms: number;
  counter: number;
  node_id: string;
}

export interface SyncConflictSummary {
  id: string;
  table: string;
  record_id: string;
  local_hlc: SyncConflictHlc;
  remote_hlc: SyncConflictHlc;
  winner: string;
  created_at: string;
}

export interface SyncConflictDetail {
  id: string;
  table: string;
  record_id: string;
  local_hlc: SyncConflictHlc;
  remote_hlc: SyncConflictHlc;
  local_data: unknown;
  remote_data: unknown;
  remote_deleted: boolean;
  winner: string;
  created_at: string;
}

export type SyncConflictStrategy = 'keep_local' | 'keep_remote' | 'dismiss';

export interface SyncTableResult {
  table: string;
  examined: number;
  applied: number;
  skipped: number;
}

export interface SyncResult {
  summary: string;
  examined: number;
  applied: number;
  skipped: number;
  conflicts: SyncConflict[];
  per_table: SyncTableResult[];
}

export interface AccountInfo {
  id: string;
  name: string;
  salt?: string;
  verifyHash?: string;
  passwordHint?: string;
  createdAt?: string;
  /** 该账户是否曾在卸载前启用过生物识别（指纹/人脸），引导用户重新设置。 */
  hasBiometricHistory?: boolean;
  /** 该账户是否曾在卸载前启用过 PIN 码解锁。 */
  hasPinHistory?: boolean;
}

export interface Profile {
  id: string;
  name: string;
  data: number[];
  createdAt: string;
  updatedAt: string;
  version: number;
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
