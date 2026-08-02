import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { FileText, Info, Folder, LayoutTemplate, RotateCcw } from 'lucide-react';
import { isMobilePlatformSync } from '@/lib/platform';
import { PluginBadge } from '@/components/template/PluginBadge';
import type { TrashItemSummary } from '@/stores/trashStore';
import { ICON_SIZE } from '@/lib/constants';

function timeAgo(ms: number, t: (k: string) => string): string {
  const diff = Date.now() - ms;
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return t('time_minutes_ago').replace('{n}', String(mins));
  const hours = Math.floor(mins / 60);
  if (hours < 24) return t('time_hours_ago').replace('{n}', String(hours));
  const days = Math.floor(hours / 24);
  if (days < 30) return t('time_days_ago').replace('{n}', String(days));
  return t('time_months_ago').replace('{n}', String(Math.floor(days / 30)));
}

interface TrashItemCardProps {
  item: TrashItemSummary;
  isSelected: boolean;
  onOpenDetail: (trashId: string) => void;
  onRestore: (trashId: string) => void;
  onDelete: (trashId: string) => void;
  onToggle: (trashId: string) => void;
}

/**
 * P119: 回收站条目卡片。memo 化后搜索击键/单选切换等状态变化
 * 只重渲染受影响的卡片（item 引用稳定 + 回调为父级稳定 useCallback），
 * 不再重建全部条目。
 */
export const TrashItemCard = memo(function TrashItemCard({
  item,
  isSelected,
  onOpenDetail,
  onRestore,
  onDelete,
  onToggle,
}: TrashItemCardProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);
  const isMobile = isMobilePlatformSync();

  const Icon =
    item.itemType === 'template' ? LayoutTemplate : item.itemType === 'page' ? Folder : FileText;

  const meta = (
    <div style={{ minWidth: 0, flex: 1 }}>
      <div
        style={{
          fontSize: 'var(--text-body)',
          fontWeight: 500,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
      >
        {item.name}
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 4,
          fontSize: 'var(--text-caption)',
          color: 'var(--text-tertiary)',
        }}
      >
        {t(`settings:trash_type_${item.itemType}`)} · {timeAgo(item.deletedAt, t)}
        {item.expiresAt &&
          ` · ${t('settings:trash_expires_in', { days: Math.max(0, Math.floor((item.expiresAt - Date.now()) / 86400000)) })}`}
        <PluginBadge contractTypeId={item.contractTypeId} size="sm" />
      </div>
    </div>
  );

  const actions = (
    <>
      <Button
        size="sm"
        variant="tertiary"
        onClick={(e) => {
          e.stopPropagation();
          onRestore(item.id);
        }}
        title={t('common:restore')}
      >
        <RotateCcw size={ICON_SIZE.sm} />
      </Button>
      <DeleteButton
        onClick={(e) => {
          e.stopPropagation();
          onDelete(item.id);
        }}
        title={t('common:delete_permanently')}
      />
      <button
        onClick={(e) => {
          e.stopPropagation();
          onOpenDetail(item.id);
        }}
        className="interactive-accent"
        style={{
          border: 'none',
          cursor: 'pointer',
          padding: 4,
          borderRadius: 4,
        }}
        title={t('common:details')}
      >
        <Info size={ICON_SIZE.lg} />
      </button>
    </>
  );

  const checkbox = (
    <SelectCheckbox
      checked={isSelected}
      onClick={(e) => {
        e.stopPropagation();
        onToggle(item.id);
      }}
    />
  );

  return (
    <Card
      interactive
      onClick={() => onOpenDetail(item.id)}
      style={{
        cursor: 'pointer',
        transition: 'transform 0.15s ease, box-shadow 0.15s ease',
      }}
    >
      {isMobile ? (
        <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
          {checkbox}
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 8,
              flex: 1,
              minWidth: 0,
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Icon size={ICON_SIZE.xl} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
              {meta}
            </div>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>{actions}</div>
          </div>
        </div>
      ) : (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {checkbox}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              flex: 1,
              minWidth: 0,
            }}
          >
            <Icon size={ICON_SIZE.xl} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
            {meta}
          </div>
          {actions}
        </div>
      )}
    </Card>
  );
});
