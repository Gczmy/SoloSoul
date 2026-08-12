import { useNavigate, useLocation } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Button } from '@/components/ui/Button';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { FilterChipGroup } from '@/components/ui/FilterChipGroup';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Search, Trash2, RotateCcw, Info } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { TrashDetailPanel } from '@/components/trash/TrashDetailPanel';
import { TrashConfirmDialog } from '@/components/trash/TrashConfirmDialog';
import { TrashItemCard } from '@/components/trash/TrashItemCard';
import { ICON_SIZE } from '@/lib/constants';

import { useTrashPage, TIME_OPTIONS, TYPE_OPTIONS, TRASH_PAGE_SIZE } from './useTrashPage';

/**
 * 回收站页 — P046 拆分后为纯展示组合层：
 * store 装配、过滤/分页、批量恢复/删除确认流、详情加载
 * 均收敛于 useTrashPage 数据 hook；本组件仅负责 JSX 组合与子组件装配。
 */
export function TrashPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const {
    t,
    // store 状态
    items,
    timeFilter,
    typeFilter,
    searchQuery,
    isLoading,
    error,
    selectedIds,
    setTimeFilter,
    setTypeFilter,
    setSearchQuery,
    // 派生
    filtered,
    allFilteredSelected,
    hasSelection,
    visibleLimit,
    setVisibleLimit,
    trashGuidePages,
    // 详情
    detailItem,
    setDetailItem,
    detailTemplate,
    detailError,
    setDetailError,
    // 确认流
    confirmAction,
    setConfirmAction,
    // 操作
    doRestore,
    doDelete,
    handleRestoreOne,
    handleDeleteOne,
    openDetail,
    toggleSelection,
    selectAll,
    clearSelection,
    handleConfirmAction,
  } = useTrashPage();

  return (
    <AppShell
      title={t('settings:trash')}
      actions={<PageGuideButton pages={trashGuidePages} />}
      onBack={() => {
        const state = location.state as { fromHome?: boolean } | undefined;
        if (state?.fromHome) {
          navigate('/home');
        } else {
          navigate('/settings');
        }
      }}
    >
      <PageContainer variant="medium" gap="default">

        <Input
          placeholder={t('settings:search_trash')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={() => setSearchQuery('')}
          prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
        />

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <FilterChipGroup
            options={TIME_OPTIONS.map((opt) => ({
              id: opt.value,
              label: t(`settings:${opt.labelKey}`, opt.labelKey),
            }))}
            value={timeFilter}
            onChange={(v) => {
              if (v) setTimeFilter(v);
            }}
          />
          <FilterChipGroup
            options={TYPE_OPTIONS.map((opt) => ({
              id: opt.value,
              label: t(`settings:trash_type_${opt.value}`),
            }))}
            value={typeFilter}
            onChange={(v) => {
              if (v) setTypeFilter(v);
            }}
          />
        </div>

        {filtered.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                fontSize: 'var(--text-sm)',
                color: 'var(--text-secondary)',
                padding: '4px 0',
              }}
            >
              <SelectCheckbox
                checked={allFilteredSelected}
                indeterminate={!allFilteredSelected && hasSelection}
                onChange={() =>
                  allFilteredSelected ? clearSelection() : selectAll(filtered.map((i) => i.id))
                }
              />
              <span>
                {t('settings:select_all')} ({filtered.length})
              </span>
            </div>

            {/* 批量操作栏：嵌入页面内，紧贴全选勾选框下方 */}
            {hasSelection && (
              <div
                style={{
                  padding: '10px 14px',
                  background: 'var(--bg-toolbar)',
                  borderRadius: 10,
                  border: '1px solid var(--border-subtle)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                }}
              >
                <Button
                  size="sm"
                  variant="tertiary"
                  onClick={() => doRestore(Array.from(selectedIds))}
                >
                  <RotateCcw size={ICON_SIZE.xs} style={{ marginRight: 4 }} />
                  {t('common:restore_all')}
                </Button>
                <DeleteButton
                  onClick={() => doDelete(Array.from(selectedIds))}
                  title={t('common:delete_permanently_all')}
                >
                  {t('common:delete_permanently_all')}
                </DeleteButton>
              </div>
            )}
          </div>
        )}

        {error && (
          <Card>
            <p
              style={{
                textAlign: 'center',
                color: 'var(--error)',
                padding: 16,
                fontSize: 'var(--text-body-sm)',
              }}
            >
              {error}
            </p>
          </Card>
        )}

        {isLoading && items.length === 0 ? (
          <Card>
            <LoadingPlaceholder variant="elevated" minHeight={120} />
          </Card>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
            {filtered.length === 0 ? (
              <Card>
                <div style={{ textAlign: 'center', padding: '48px 24px' }}>
                  <Trash2
                    size={ICON_SIZE['5xl']}
                    style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }}
                  />
                  <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
                    {items.length > 0
                      ? t('settings:trash_empty_filtered')
                      : t('settings:trash_empty')}
                  </p>
                </div>
              </Card>
            ) : (
              <>
                {filtered.slice(0, visibleLimit).map((item) => (
                  <TrashItemCard
                    key={item.id}
                    item={item}
                    isSelected={selectedIds.has(item.id)}
                    onOpenDetail={openDetail}
                    onRestore={handleRestoreOne}
                    onDelete={handleDeleteOne}
                    onToggle={toggleSelection}
                  />
                ))}
                {filtered.length > visibleLimit && (
                  <Button
                    variant="tertiary"
                    size="sm"
                    onClick={() => setVisibleLimit((n) => n + TRASH_PAGE_SIZE)}
                    style={{ marginTop: 4 }}
                  >
                    {t('common:load_more', { defaultValue: '加载更多' })}
                  </Button>
                )}
              </>
            )}
          </div>
        )}

        {detailItem && (
          <TrashDetailPanel
            detailItem={detailItem}
            detailTemplate={detailTemplate}
            onClose={() => {
              setDetailItem(null);
              setDetailError(null);
            }}
            onRequestRestore={(id) => doRestore([id])}
            onRequestDelete={(id) => doDelete([id])}
          />
        )}

        {detailError && !detailItem && (
          <Card>
            <div
              style={{
                textAlign: 'center',
                padding: '32px 24px',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 12,
              }}
            >
              <Info size={ICON_SIZE['2xl']} style={{ opacity: 0.4, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
                {t('settings:trash_detail_load_failed', { defaultValue: '加载回收站详情失败' })}
              </p>
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  maxWidth: 420,
                  wordBreak: 'break-word',
                }}
              >
                {detailError.message}
              </p>
              <Button
                variant="primary"
                size="sm"
                onClick={() => openDetail(detailError.trashId)}
              >
                {t('common:retry')}
              </Button>
            </div>
          </Card>
        )}

        {confirmAction && (
          <TrashConfirmDialog
            action={confirmAction}
            onClose={() => setConfirmAction(null)}
            onConfirm={() => handleConfirmAction(confirmAction)}
          />
        )}
      </PageContainer>
    </AppShell>
  );
}
