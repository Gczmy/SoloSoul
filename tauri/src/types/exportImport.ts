/** 真实 IPC 形状由 Rust 生成；本文件只保留 UI 摘要及状态。 */
import type {
  ObjectSummary,
  PageGroup as PageGroupWire,
  DecryptedImportPreview as DecryptedImportPreviewWire,
  ImportOperationSummary,
} from '@/lib/generated/ipcContracts';

export type {
  AttachmentInfo,
  AttachmentImportInfo,
  ExportEstimate,
  ImportPreview,
  ConflictKind,
  ConflictInfo,
  ImportResult,
  DocumentSensitivity,
  ExportDocumentResult,
  AdvancedImportRequestInput as AdvancedImportRequest,
  ImportStrategyInput as ImportStrategy,
  ImportOperationSummary,
} from '@/lib/generated/ipcContracts';

/** 范围树展示所需的局部摘要；完整 wire ObjectSummary 不被缩减。 */
export type ExportObjectSummary = Pick<
  ObjectSummary,
  'id' | 'name' | 'typeId' | 'sectionType' | 'sensitivityLevel' | 'createdAt' | 'updatedAt' | 'tags'
> &
  Partial<Pick<ObjectSummary, 'sensitivityLevels' | 'hasAttachments'>>;
export type PageGroup = Omit<PageGroupWire, 'objects'> & { objects: ExportObjectSummary[] };
export type DecryptedImportPreview = Omit<DecryptedImportPreviewWire, 'objects'> & {
  objects: ExportObjectSummary[];
};

/** 云盘同步目标（Phase 1 云打包，Rust cloud_targets_detect）。 */
export interface CloudTargetInfo {
  id: string;
  name: string;
  path: string;
}

/** 仅 UI 状态与动作；不是 IPC DTO，不保存密码到持久 Store。 */
export interface ImportOperationsUi {
  items: ImportOperationSummary[];
  selected: ImportOperationSummary | null;
  currentId: string | null;
  loading: boolean;
  loadingDetails: boolean;
  busy: boolean;
  password: string;
  replacementSource: string;
  canResume: boolean;
  onRefresh: () => Promise<void>;
  onSelect: (id: string) => Promise<void>;
  onRetry: () => Promise<void>;
  onResume: () => Promise<void>;
  onSetPassword: (value: string) => void;
  onPickSource: () => Promise<void>;
  onContinueLater: () => void;
  onNewImport: () => void;
}
