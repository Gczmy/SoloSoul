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

export interface DecryptedImportPreview {
  objects: ObjectSummary[];
  conflicts: ConflictInfo[];
  hasPreferences: boolean;
  hasAuditLog: boolean;
}

export interface ConflictInfo {
  objectId: string;
  name: string;
}

export type ImportStrategy = 'skipExisting' | 'overwrite' | 'merge';

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
  let score = 0;
  if (pw.length >= 8) score++;
  if (pw.length >= 12) score++;
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) score++;
  if (/\d/.test(pw)) score++;
  if (/[^a-zA-Z0-9]/.test(pw)) score++;
  if (score <= 1) return 'weak';
  if (score <= 3) return 'medium';
  return 'strong';
}
