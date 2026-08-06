import { useState, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useToastError } from '@/hooks/useToastError';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { saveWithPause } from '@/lib/dialog';
import { Search, Download, X } from 'lucide-react';
import { useSettingsStore } from '@/stores/settingsStore';
import { FilterChipGroup } from '@/components/ui/FilterChipGroup';
import { ICON_SIZE } from '@/lib/constants';
import { isUriPath, copyStagedFileToDest } from '@/lib/mobileFileTransfer';
import { OperationLogCard } from '@/components/settings/OperationLogCard';
import type { AuditLogEntry } from '@/components/settings/OperationLogCard';
import buttonStyles from '@/components/ui/Button.module.css';

/** All known entity types — used for filter buttons */
const ALL_ENTITY_TYPES = [
  'object',
  'page',
  'preference',
  'profile',
  'biometric',
  'template',
  'export',
  'import',
  'attachment',
  'trash_item',
  'llm',
  'auth',
  'file',
  'ocr_model',
  'sync',
];

/** P218: 单次渲染上限（「加载更多」步进量）。 */
const LOG_PAGE_SIZE = 50;

export function OperationLogPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [entityTypeFilter, setEntityTypeFilter] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  // P218: 分页「加载更多」，避免 200 条全量挂载。
  const [visibleLimit, setVisibleLimit] = useState(LOG_PAGE_SIZE);

  const loadLogs = useCallback(async () => {
    setIsLoading(true);
    try {
      const entries = await invoke<AuditLogEntry[]>('log_get_recent', { limit: 200 });
      setLogs(entries);
    } catch (e) {
      onError(e, t('common:logs_load_failed'));
    } finally {
      setIsLoading(false);
    }
  }, [onError, t]);

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  // P218: 搜索词或筛选变更时重置分页游标。
  useEffect(() => {
    setVisibleLimit(LOG_PAGE_SIZE);
  }, [searchQuery, entityTypeFilter]);

  const handleExport = async () => {
    try {
      const filePath = await saveWithPause({
        defaultPath: 'audit_log_export.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!filePath) return; // User cancelled

      // Android 保存对话框返回 content:// URI，Rust 只能写到应用私有目录，
      // 先导出到默认位置再用 plugin-fs 中转。
      if (isUriPath(filePath)) {
        const exportedPath = await invoke<string>('log_export', {});
        await copyStagedFileToDest(exportedPath, filePath);
        onSuccess(`${t('common:export')} → ${filePath}`);
      } else {
        const result = await invoke<string>('log_export', { exportPath: filePath });
        onSuccess(`${t('common:export')} → ${result}`);
      }
    } catch (e) {
      onError(e, t('common:logs_export_failed'));
    }
  };

  // P218: useMemo 缓存过滤结果——搜索击键只重建数组，entry 引用不变，
  // memo 化的 OperationLogCard 全部跳过重渲染。
  const filteredLogs = useMemo(() => {
    return logs.filter((entry) => {
      if (entityTypeFilter && entry.entityType !== entityTypeFilter) return false;
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        return (
          entry.actionType.toLowerCase().includes(q) ||
          entry.entityType.toLowerCase().includes(q) ||
          (entry.entityName?.toLowerCase().includes(q) ?? false) ||
          (entry.details?.toLowerCase().includes(q) ?? false)
        );
      }
      return true;
    });
  }, [logs, entityTypeFilter, searchQuery]);

  const visibleLogs = filteredLogs.slice(0, visibleLimit);

  return (
    <AppShell title={t('settings:operation_log')} onBack={() => navigate('/settings')}>
      <PageContainer variant="wide" gap="default">
        {/* Row 1: Search + Export */}
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <div
            style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              border: '1px solid var(--border-subtle)',
              borderRadius: 8,
              padding: '0 10px',
            }}
          >
            <Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('settings:search_logs')}
              style={{
                flex: 1,
                border: 'none',
                outline: 'none',
                padding: '8px 4px',
                fontSize: 'var(--text-sm)',
                background: 'transparent',
                color: 'var(--text-primary)',
                fontFamily: 'inherit',
              }}
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                aria-label={t('common:clear')}
                tabIndex={-1}
                className="interactive-accent"
                style={{
                  flexShrink: 0,
                  width: 22,
                  height: 22,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  border: 'none',
                  borderRadius: 4,
                  cursor: 'pointer',
                  padding: 0,
                }}
              >
                <X size={ICON_SIZE.sm} />
              </button>
            )}
          </div>
          <Button
            variant="secondary"
            size="sm"
            className={buttonStyles.hideLabelOnMobile}
            aria-label={t('settings:export_logs', { defaultValue: 'Export logs' })}
            onClick={handleExport}
          >
            <Download size={ICON_SIZE.sm} />{' '}
            <span className={buttonStyles.label}>{t('settings:export_logs')}</span>
          </Button>
        </div>

        {/* Row 2: Entity type filter — workspace style */}
        <FilterChipGroup
          toggle
          radius={8}
          gap={4}
          options={[null, ...ALL_ENTITY_TYPES].map((type) => ({
            id: type,
            label: type === null ? t('settings:all') : t(`settings:log.entity.${type}`, type),
          }))}
          value={entityTypeFilter}
          onChange={(v) => setEntityTypeFilter(v)}
        />

        {/* Log entries */}
        {isLoading ? (
          <LoadingPlaceholder variant="base" minHeight={200} />
        ) : visibleLogs.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: 40, color: 'var(--text-tertiary)' }}>
              <p style={{ fontSize: 'var(--text-sm)' }}>{t('settings:no_log_entries')}</p>
              <p style={{ fontSize: 'var(--text-caption)', marginTop: 4 }}>
                {searchQuery || entityTypeFilter
                  ? t('settings:adjust_filters')
                  : t('settings:logs_hint')}
              </p>
            </div>
          </Card>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
            {visibleLogs.map((entry) => (
              <OperationLogCard key={entry.id} entry={entry} customPages={customPages} />
            ))}
            {filteredLogs.length > visibleLimit && (
              <Button
                variant="tertiary"
                size="sm"
                onClick={() => setVisibleLimit((n) => n + LOG_PAGE_SIZE)}
                style={{ marginTop: 4 }}
              >
                {t('load_more', { defaultValue: '加载更多' })}
              </Button>
            )}
          </div>
        )}
      </PageContainer>
    </AppShell>
  );
}
