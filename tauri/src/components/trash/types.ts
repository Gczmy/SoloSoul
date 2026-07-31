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
}

export interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}

export interface TrashConfirmAction {
  type: 'restore' | 'delete';
  ids: string[];
  count: number;
  pageChildCount?: number;
  callback: () => Promise<void>;
}

