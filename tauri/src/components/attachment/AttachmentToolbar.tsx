import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { Paperclip, RotateCcw, Search, Download } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Input } from '@/components/ui/Input';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { ICON_SIZE } from '@/lib/constants';
import { formatBytes } from '@/lib/utils';
import buttonStyles from '@/components/ui/Button.module.css';

export interface AttachmentSummaryStats {
  activeAttachments: number;
  activeBytes: number;
  activeObjects: number;
  trashAttachments: number;
  trashBytes: number;
  trashObjects: number;
}

interface AttachmentToolbarProps {
  showTrash: boolean;
  searchQuery: string;
  loading: boolean;
  hasData: boolean;
  summary: AttachmentSummaryStats;
  visibleKeys: string[];
  selectedCount: number;
  allSelected: boolean;
  onSearchChange: (value: string) => void;
  onShowActive: () => void;
  onShowTrash: () => void;
  onRefresh: () => void;
  onSelectAll: () => void;
  onBatchDownload: () => void;
  onBatchDelete: () => void;
  onBatchRestore: () => void;
  onBatchPermanentDelete: () => void;
}

/** 搜索框 + 标签页 + 摘要卡 + 批量操作栏（P024 拆分）。 */
export function AttachmentToolbar({
  showTrash,
  searchQuery,
  loading,
  hasData,
  summary,
  visibleKeys,
  selectedCount,
  allSelected,
  onSearchChange,
  onShowActive,
  onShowTrash,
  onRefresh,
  onSelectAll,
  onBatchDownload,
  onBatchDelete,
  onBatchRestore,
  onBatchPermanentDelete,
}: AttachmentToolbarProps) {
  const { t } = useTranslation(['settings', 'common', 'navigation']);

  const { activeAttachments, activeBytes, trashAttachments, trashBytes } = summary;

  return (
    <>
      <Input
        placeholder={
          showTrash
            ? t('common:search_trash', { defaultValue: 'Search trash...' })
            : t('common:search_attachments', { defaultValue: 'Search attachments...' })
        }
        value={searchQuery}
        onChange={(e) => onSearchChange(e.target.value)}
        onClear={() => onSearchChange('')}
        prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
      />

      {!loading && hasData && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
          style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-md)' }}
        >
          {/* Tab pills */}
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <Button
              variant="secondary"
              size="sm"
              onClick={onShowActive}
              style={
                !showTrash
                  ? {
                      background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                      borderColor: 'var(--accent-primary)',
                      color: 'var(--accent-primary)',
                      boxShadow: '0 0 0 1px var(--accent-primary)',
                    }
                  : undefined
              }
            >
              {t('common:attachments_active', {
                n: activeAttachments,
                defaultValue: `Attachments (${activeAttachments})`,
              })}
              <span style={{ marginLeft: 4, fontSize: 'var(--text-caption)', opacity: 0.7 }}>
                {formatBytes(activeBytes)}
              </span>
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={onShowTrash}
              className="interactive-danger-tab"
              style={
                showTrash
                  ? {
                      background: 'color-mix(in srgb, #e74c3c 10%, transparent)',
                      borderColor: '#e74c3c',
                      color: '#e74c3c',
                      boxShadow: '0 0 0 1px #e74c3c',
                    }
                  : undefined
              }
            >
              {t('common:attachments_trash', {
                n: trashAttachments,
                defaultValue: `Trash (${trashAttachments})`,
              })}
              <span style={{ marginLeft: 4, fontSize: 'var(--text-caption)', opacity: 0.7 }}>
                {formatBytes(trashBytes)}
              </span>
            </Button>

            <div style={{ flex: 1 }} />

            <Button
              variant="secondary"
              size="sm"
              className={buttonStyles.hideLabelOnMobile}
              aria-label={t('common:refresh', { defaultValue: 'Refresh' })}
              onClick={onRefresh}
            >
              <RotateCcw size={ICON_SIZE.sm} />{' '}
              <span className={buttonStyles.label}>{t('common:refresh', { defaultValue: 'Refresh' })}</span>
            </Button>
          </div>

          {/* Summary card */}
          <Card style={{ padding: '12px 16px' }}>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 20,
                fontSize: 'var(--text-sm)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <Paperclip size={ICON_SIZE.sm} style={{ color: 'var(--accent-primary)' }} />
                <span style={{ color: 'var(--text-tertiary)' }}>{t('common:attachments')}</span>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                  {showTrash ? trashAttachments : activeAttachments}
                </span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span style={{ color: 'var(--text-tertiary)' }}>{t('common:size')}</span>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                  {formatBytes(showTrash ? trashBytes : activeBytes)}
                </span>
              </div>
              <div style={{ flex: 1 }} />
              <div style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
                {t('settings:objects_count', {
                  n: showTrash ? summary.trashObjects : summary.activeObjects,
                })}
              </div>
            </div>
          </Card>

          {/* Batch toolbar */}
          <Card style={{ padding: '8px 14px' }}>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                fontSize: 'var(--text-sm)',
              }}
            >
              <div
                onClick={onSelectAll}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  cursor: visibleKeys.length > 0 ? 'pointer' : 'default',
                  color: 'var(--text-secondary)',
                  userSelect: 'none',
                }}
              >
                <SelectCheckbox
                  checked={allSelected}
                  indeterminate={selectedCount > 0 && !allSelected}
                  disabled={visibleKeys.length === 0}
                />
              </div>

              <div style={{ flex: 1 }} />

              <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
                {t('common:selected_count', { n: selectedCount })}
              </span>

              {selectedCount > 0 && !showTrash ? (
                <div style={{ display: 'flex', gap: 6 }}>
                  <Button variant="secondary" size="sm" onClick={onBatchDownload}>
                    <Download size={ICON_SIZE.sm} /> {t('common:download')}
                  </Button>
                  <DeleteButton onClick={onBatchDelete} title={t('common:delete')}>
                    {t('common:delete')}
                  </DeleteButton>
                </div>
              ) : selectedCount > 0 && showTrash ? (
                <div style={{ display: 'flex', gap: 6 }}>
                  <Button variant="secondary" size="sm" onClick={onBatchRestore}>
                    <RotateCcw size={ICON_SIZE.sm} /> {t('common:restore')}
                  </Button>
                  <DeleteButton
                    onClick={onBatchPermanentDelete}
                    title={t('common:delete_permanently')}
                  >
                    {t('common:delete_permanently')}
                  </DeleteButton>
                </div>
              ) : null}
            </div>
          </Card>
        </motion.div>
      )}
    </>
  );
}
