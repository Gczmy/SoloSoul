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
}

export interface ProfileSummary {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  version: number;
}

export interface Profile {
  id: string;
  name: string;
  data: number[];
  createdAt: string;
  updatedAt: string;
  version: number;
}

export type VaultStateStr = 'uninitialized' | 'locked' | 'unlocked';

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
}
