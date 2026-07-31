import { useTranslation } from 'react-i18next';
import { ChevronRight, ChevronDown } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { ICON_SIZE } from '@/lib/constants';
import { formatBytes } from '@/lib/utils';
import { isMobilePlatformSync } from '@/lib/platform';
import { DEFAULT_CUSTOM_ICON, PAGE_ICON_MAP, CUSTOM_ICON_MAP } from '@/lib/pageIcons';
import type { ReactNode } from 'react';
import type { PageIconKey, CustomIconId } from '@/lib/pageIcons';
import type { LucideIcon } from 'lucide-react';
import type { AttachmentTreePage } from '@/components/attachment/attachmentManagerTypes';

/** Resolve icon from either PAGE_ICON_MAP (built-in) or CUSTOM_ICON_MAP (user-selectable). */
function resolvePageIcon(iconKey?: string | null): LucideIcon {
  const id = iconKey || DEFAULT_CUSTOM_ICON;
  if (id in PAGE_ICON_MAP) return PAGE_ICON_MAP[id as PageIconKey];
  if (id in CUSTOM_ICON_MAP) return CUSTOM_ICON_MAP[id as CustomIconId];
  return CUSTOM_ICON_MAP[DEFAULT_CUSTOM_ICON];
}

interface AttachmentPageCardProps {
  page: AttachmentTreePage;
  isExpanded: boolean;
  onToggle: () => void;
  renderObjects: () => ReactNode;
}

/** 页面分组卡片，展开时渲染其下的对象分组。 */
export function AttachmentPageCard({
  page,
  isExpanded,
  onToggle,
  renderObjects,
}: AttachmentPageCardProps) {
  const { t } = useTranslation(['settings', 'common', 'navigation']);
  const PageIconComp = resolvePageIcon(page.pageIcon);
  const isMobile = isMobilePlatformSync();

  const totalAttachments = page.objects.reduce((sum, o) => sum + o.attachments.length, 0);
  const totalBytes = page.objects.reduce(
    (sum, o) => sum + o.attachments.reduce((s, a) => s + a.sizeBytes, 0),
    0,
  );
  const pageLabel = page.pageId ? page.pageName : t(`navigation:${page.pageName}`);

  return (
    <Card style={{ padding: 0, overflow: 'hidden' }}>
      <div
        onClick={onToggle}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '10px 14px',
          cursor: 'pointer',
          fontSize: 'var(--text-sm)',
          fontWeight: 600,
          color: 'var(--text-primary)',
          background: 'var(--bg-toolbar)',
          borderBottom: isExpanded ? '1px solid var(--border-subtle)' : 'none',
          transition: 'background 0.15s',
        }}
        className="interactive-toolbar"
      >
        <PageIconComp
          size={ICON_SIZE.xl}
          style={{ flexShrink: 0, color: 'var(--accent-primary)' }}
        />
        {isMobile ? (
          // 移动端：第1行 页面名+展开箭头，第2行 统计信息
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span
                style={{
                  flex: 1,
                  minWidth: 0,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {pageLabel}
              </span>
              {isExpanded ? (
                <ChevronDown size={ICON_SIZE.sm} style={{ flexShrink: 0 }} />
              ) : (
                <ChevronRight size={ICON_SIZE.sm} style={{ flexShrink: 0 }} />
              )}
            </div>
            <div
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                marginTop: 2,
              }}
            >
              {t('settings:objects_count', { n: page.objects.length })} ·{' '}
              {t('settings:attachments_count', { n: totalAttachments })} · {formatBytes(totalBytes)}
            </div>
          </div>
        ) : (
          // 桌面端：单行布局
          <>
            <span style={{ flex: 1 }}>{pageLabel}</span>
            <span
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                whiteSpace: 'nowrap',
              }}
            >
              {t('settings:objects_count', { n: page.objects.length })} ·{' '}
              {t('settings:attachments_count', { n: totalAttachments })} · {formatBytes(totalBytes)}
            </span>
            {isExpanded ? (
              <ChevronDown size={ICON_SIZE.sm} />
            ) : (
              <ChevronRight size={ICON_SIZE.sm} />
            )}
          </>
        )}
      </div>
      {isExpanded && renderObjects()}
    </Card>
  );
}
