export type SensitivityLevel = 'public' | 'internal' | 'sensitive' | 'critical';

export interface PageGroup {
  sectionType: string;
  pageName: string;
  objectCount: number;
  objects: ObjectSummary[];
}

export interface ObjectSummary {
  id: string;
  name: string;
  collectionType: string;
  sectionType: string;
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
  tags: string[];
}

export interface AttachmentInfo {
  id: string;
  fileName: string;
  sizeBytes: number;
}

export interface ExportEstimate {
  objectCount: number;
  attachmentCount: number;
  attachmentSelectedCount: number;
  estimatedBytes: number;
}

export interface ImportPreview {
  filePath: string;
  version: string;
  objectCount: number;
  hasAttachments: boolean;
  extraFiles: string[];
  exportTime: string | null;
  passwordHint: string | null;
}

export interface AttachmentImportInfo {
  id: string;
  objectId: string;
  fileName: string;
  sizeBytes: number;
}

export interface DecryptedImportPreview {
  objects: ObjectSummary[];
  conflicts: ConflictInfo[];
  hasPreferences: boolean;
  hasAuditLog: boolean;
  attachments: AttachmentImportInfo[];
}

export type ConflictKind = 'identical' | 'renamedLocal';

export interface ConflictInfo {
  objectId: string;
  importedName: string;
  existingName: string;
  kind: ConflictKind;
}

export type ImportStrategy = 'skipExisting' | 'overwrite' | 'keepBoth';

export interface ImportSelection {
  objectId: string;
  selected: boolean;
}

export interface ImportResult {
  objectCount: number;
  attachmentCount: number;
}

export type PasswordStrength = 'none' | 'weak' | 'medium' | 'strong';

export function assessPasswordStrength(pw: string): PasswordStrength {
  if (!pw) return 'none';
  // 不再限制密码复杂度 — 由用户自行决定密码难度
  return 'strong';
}
