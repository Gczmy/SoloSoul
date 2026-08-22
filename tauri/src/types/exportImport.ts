export interface PageGroup {
  sectionType: string;
  pageName: string;
  objectCount: number;
  objects: ExportObjectSummary[];
}

/**
 * 导出/导入范围树中的对象摘要（P037：与 workspace 的 ObjectSummary 语义不同，
 * 原名相同易混淆——重命名为 ExportObjectSummary）。
 */
export interface ExportObjectSummary {
  id: string;
  name: string;
  typeId: string;
  sectionType: string;
  sensitivityLevel: string;
  /**
   * 对象各字段的敏感度等级集合（升序：public < internal < sensitive < critical）。
   * 由后端从 property_labels / __fields / 模板定义推导；范围树据此展示字段敏感度徽章，
   * 缺省（旧后端/导入预览）时回退到 sensitivityLevel 单徽章。
   */
  sensitivityLevels?: string[];
  createdAt: string;
  updatedAt: string;
  tags: string[];
  /** 该对象是否包含（未软删的）附件——导出范围树据此决定是否展示附件展开图标 */
  hasAttachments?: boolean;
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
  objects: ExportObjectSummary[];
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

export interface ImportResult {
  objectCount: number;
  attachmentCount: number;
}

/** 文档导出 preflight 返回的字段最高敏感度（Rust export_document_preflight）。 */
export type DocumentSensitivity = 'none' | 'sensitive' | 'critical';

/** 文档导出结果（Rust export_objects_document）。 */
export interface ExportDocumentResult {
  objectCount: number;
  fileSizeBytes: number;
}

/** 云盘同步目标（Phase 1 云打包，Rust cloud_targets_detect）。 */
export interface CloudTargetInfo {
  id: string;
  name: string;
  path: string;
}
