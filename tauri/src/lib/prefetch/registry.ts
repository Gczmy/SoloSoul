/**
 * 预取注册表（Prefetch Runtime，docs/prefetch-runtime-design.md）。
 *
 * 所有页面级异步数据的统一登记处。P1 试点：OCR 模型状态（OcrPage /
 * OcrSettingsPage 共用，进入页面直接命中缓存 → 无骨架期）。
 * P2 起按批次追加 vault stats / backups / templates / trash / syncStatus 等。
 */
import { createPrefetchStore } from './createPrefetchStore';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { isMobilePlatformSync } from '@/lib/platform';
import { useAuthStore } from '@/stores/authStore';
import type { OcrTierInfo, OcrModelStatus } from '@/lib/ipc';
import type { VaultStats } from '@/pages/settings/StorageBreakdownCard';
import type { BackupInfo } from '@/types/backup';
import type { AuditLogEntry } from '@/types/auditLog';
import type { PageGroup } from '@/types/exportImport';

export interface OcrModelState {
  tiers: OcrTierInfo[];
  activeTier: string;
  statusMap: Record<string, OcrModelStatus>;
}

/** 一次拉齐 OCR 模型全量状态（list tiers + active tier + 每档 status）。 */
async function loadOcrModelState(): Promise<OcrModelState> {
  const [tierList, currentTier] = await Promise.all([
    invoke<OcrTierInfo[]>('ocr_list_available_tiers'),
    invoke<string>('ocr_get_active_tier'),
  ]);
  const statuses: Record<string, OcrModelStatus> = {};
  await Promise.all(
    tierList.map(async (tier) => {
      statuses[tier.tier] = await invoke<OcrModelStatus>('ocr_get_model_status', {
        tier: tier.tier,
      });
    }),
  );
  return { tiers: tierList, activeTier: currentTier, statusMap: statuses };
}

export const prefetchRegistry = {
  /** OCR 模型状态：移动端用系统 ML Kit 不渲染模型卡片，跳过预热。 */
  ocrModel: createPrefetchStore<OcrModelState>({
    key: 'ocr-model',
    loader: loadOcrModelState,
    ttlMs: 5 * 60_000,
    warmupPolicy: 'afterAuth',
    enabledOnPlatform: () => !isMobilePlatformSync(),
  }),
  /** 保险库统计（设置页 + 数据管理页共用）。 */
  vaultStats: createPrefetchStore<VaultStats>({
    key: 'vault-stats',
    loader: () => invoke<VaultStats>('get_vault_stats'),
    ttlMs: 60_000,
    warmupPolicy: 'afterAuth',
  }),
  /** 备份列表（备份/恢复页）。 */
  backups: createPrefetchStore<BackupInfo[]>({
    key: 'backups',
    loader: () => invoke<BackupInfo[]>('backup_list'),
    ttlMs: 60_000,
    warmupPolicy: 'afterAuth',
  }),
  /** 操作审计日志（调试日志页 + 操作日志页共用，跨页共享缓存；不预热）。 */
  logs: createPrefetchStore<AuditLogEntry[]>({
    key: 'logs',
    loader: () => invoke<AuditLogEntry[]>('log_get_recent', { limit: 200 }),
    ttlMs: 60_000,
    warmupPolicy: 'never',
  }),
  /** 导出范围树（导入导出页的导出/导出为文档两个 tab 共用；导入成功后 invalidate 刷新）。 */
  exportScope: createPrefetchStore<PageGroup[]>({
    key: 'export-scope',
    loader: async () => {
      const accountId = useAuthStore.getState().currentAccount?.id;
      if (!accountId) throw new Error('No account is currently unlocked');
      return invoke<PageGroup[]>('export_get_scope_tree', { accountId });
    },
    ttlMs: 60_000,
    warmupPolicy: 'afterAuth',
  }),
};
