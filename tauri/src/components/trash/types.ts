import type { PropertyType, SensitivityLevel } from '@/types/template';

export interface TrashDetail {
  id: string;
  itemType: string;
  originalId: string;
  name: string;
  sectionType?: string;
  deletedAt: number;
  expiresAt?: number;
  deletedBy: string;
  remainingDays?: number;
  originalLocation: string;
  templateId?: string;
  propertyLabels?: Record<string, SensitivityLevel>;
  previewProperties: {
    fieldId?: string;
    key: string;
    value: unknown;
    type?: PropertyType;
    sensitivityLevel?: SensitivityLevel;
  }[];
  attachments: TrashAttachment[];
  deletedAttachments: TrashAttachment[];
  snapshots: SnapshotEntry[];
  childItems: TrashChildSummary[];
}

export interface TrashChildSummary {
  id: string;
  originalId: string;
  name: string;
  itemType: string;
}

export interface TrashAttachment {
  id: string;
  fileName: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
  deletedAt?: string | null;
  /** 附件描述（随快照携带；旧数据可能缺失） */
  description?: string | null;
  /** 附件标签（随快照携带；旧数据可能缺失） */
  tags?: string[];
  /** 落库副本路径（后端 exists 探测：文件仍在磁盘才返回；已删/旧数据为 null） */
  vaultPath?: string | null;
}

import type { SnapshotEntry } from '@/types/history';
export type { SnapshotEntry };

export interface TrashConfirmAction {
  type: 'restore' | 'delete';
  ids: string[];
  count: number;
  pageChildCount?: number;
  callback: () => Promise<void>;
}
