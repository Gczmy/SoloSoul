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
  /** 随本次导出打包的用户模板快照数量与名称（后端与导出执行同一收集逻辑） */
  templateCount: number;
  templateNames: string[];
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
