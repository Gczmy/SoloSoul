/**
 * 备份条目（P037 收敛单一来源）。
 * 此前 BackupConfigPage.tsx 定义完整形状、notification.ts 另定义最小子集。
 */
export interface BackupInfo {
  id: string;
  name: string;
  created_at: string;
  size_bytes: number;
  object_count: number;
}
