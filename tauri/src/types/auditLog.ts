/**
 * 操作审计日志条目（P037 收敛单一来源）。
 * 此前在 OperationLogCard.tsx 与 DebugLogPage.tsx 各定义一份。
 */
export interface AuditLogEntry {
  id: number;
  timestamp: string;
  actionType: string;
  entityType: string;
  entityId: string | null;
  entityName: string | null;
  performedBy: string;
  details: string | null;
}
