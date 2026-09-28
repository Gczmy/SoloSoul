/** 模板同步 wire 类型由 Rust 生成；仅保留前端同步判断所需的派生视图。 */
export type {
  SyncFieldInfo,
  SyncFieldChangeItem,
  SyncFieldChange,
  SyncFieldIncompatible,
  TemplateSyncResult,
  DeprecatedField,
} from '@/lib/generated/ipcContracts';
import type { ObjectSummaryView } from '@/lib/objectViewModel';

/** Minimal object info required by sync checks. */
export type SyncableObject = Pick<
  ObjectSummaryView,
  'id' | 'templateId' | 'templateHash' | 'ignoredTemplateHash'
>;
