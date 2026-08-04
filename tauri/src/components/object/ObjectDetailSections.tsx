import { X, Clock, Paperclip, Pencil, type LucideIcon } from 'lucide-react';
import type { useTranslation } from 'react-i18next';
import type { ObjectData, ObjectSummary } from '@/stores/objectStore';
import { PluginBadge } from '@/components/template/PluginBadge';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import type { GuidePage } from '@/components/guide/PageGuide';
import { ICON_SIZE } from '@/lib/constants';
import styles from './ObjectDetailModal.module.css';

type T = ReturnType<typeof useTranslation>['t'];

/** 详情卡片头部：图标 + 名称 + 元信息（页面/模板/插件徽章/创建更新时间）+ 关闭。 */
export function ObjectDetailHeader({
  obj,
  icon: ObjectDetailIcon,
  detailTplMatch,
  detailTplName,
  collectionLabel,
  t,
  onClose,
}: {
  obj: ObjectData | ObjectSummary;
  icon: LucideIcon;
  detailTplMatch: boolean;
  detailTplName?: string;
  collectionLabel: string;
  t: T;
  onClose: () => void;
}) {
  return (
    <div
      className={styles.modalHeader}
      style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ flexShrink: 0, display: 'flex' }}>
          <ObjectDetailIcon size={ICON_SIZE['2xl']} />
        </span>
        <div>
          <h2
            style={{
              fontSize: 'var(--text-md)',
              fontWeight: 700,
              margin: 0,
              overflowWrap: 'break-word',
              wordBreak: 'break-word',
            }}
          >
            {obj.name}
          </h2>
          <span
            style={{
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              wordBreak: 'break-word',
            }}
          >
            {collectionLabel}
            {/* 模板名 — 模板不匹配（已删除/更改页面）时显示删除线 */}
            {obj.templateId &&
              (() => {
                const tplName = (obj.properties as Record<string, unknown>)?.__templateName as
                  | string
                  | undefined;
                const tid = obj.templateId || '';
                const label = detailTplMatch
                  ? detailTplName || tid
                  : tplName
                    ? `${tplName} (${tid.slice(0, 8)}…)`
                    : tid;
                return (
                  <span style={{ textDecoration: detailTplMatch ? 'none' : 'line-through' }}>
                    {' · '}
                    {label}
                  </span>
                );
              })()}
            {obj.contractTypeId && (
              <span style={{ marginLeft: 4, display: 'inline-flex', verticalAlign: 'middle' }}>
                <PluginBadge contractTypeId={obj.contractTypeId} size="sm" variant="full" />
              </span>
            )}
            {' · '}
            {t('common:created')}: {obj.createdAt?.slice(0, 10) || '—'} · {t('common:updated')}:{' '}
            {obj.updatedAt?.slice(0, 10) || '—'}
          </span>
        </div>
      </div>
      <button
        onClick={onClose}
        className={styles.closeBtn}
        data-testid="object-detail-close"
        aria-label={t('common:close')}
      >
        <X size={ICON_SIZE.xl} />
      </button>
    </div>
  );
}

/** 模板已更新且对象尚未同步时的提示条。 */
export function ObjectDetailTemplateSyncBanner({
  t,
  onSync,
  onDismiss,
}: {
  t: T;
  onSync: () => void;
  onDismiss: (() => void) | undefined;
}) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 12,
        marginBottom: 16,
        padding: '10px 12px',
        borderRadius: 8,
        background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
        border: '1px solid color-mix(in srgb, var(--accent-primary) 25%, transparent)',
      }}
    >
      <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-primary)' }}>
        {t('editor:template_updated_hint')}
      </span>
      <div style={{ display: 'flex', gap: 8, flexShrink: 0 }}>
        <button
          onClick={onSync}
          style={{
            padding: '4px 10px',
            borderRadius: 6,
            border: 'none',
            background: 'var(--accent-primary)',
            color: '#fff',
            fontSize: 'var(--text-caption)',
            fontWeight: 600,
            cursor: 'pointer',
          }}
        >
          {t('common:yes')}
        </button>
        <button
          onClick={() => onDismiss?.()}
          style={{
            padding: '4px 10px',
            borderRadius: 6,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-secondary)',
            fontSize: 'var(--text-caption)',
            fontWeight: 500,
            cursor: 'pointer',
          }}
        >
          {t('common:no')}
        </button>
      </div>
    </div>
  );
}

/** 查看已归档历史字段入口按钮。 */
export function ObjectDetailDeprecatedEntry({
  t,
  count,
  onView,
}: {
  t: T;
  count: number;
  onView: () => void;
}) {
  return (
    <div style={{ marginBottom: 12 }}>
      <button
        onClick={onView}
        style={{
          padding: '6px 10px',
          borderRadius: 6,
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-toolbar)',
          color: 'var(--text-secondary)',
          fontSize: 'var(--text-caption)',
          cursor: 'pointer',
        }}
      >
        {t('editor:deprecated_fields_button', { count })}
      </button>
    </div>
  );
}

/** 对象标签 Pills。 */
export function ObjectDetailTags({ tags }: { tags: string[] }) {
  return (
    <div style={{ marginTop: 16, display: 'flex', gap: 6, flexWrap: 'wrap' }}>
      {tags.map((tag: string) => (
        <span
          key={tag}
          style={{
            padding: '2px 8px',
            borderRadius: 10,
            fontSize: 'var(--text-badge)',
            background: 'var(--bg-toolbar)',
            color: 'var(--text-secondary)',
            border: '1px solid var(--border-subtle)',
          }}
        >
          {tag}
        </span>
      ))}
    </div>
  );
}

/** 详情卡片底部操作栏：指南 + 历史/附件/编辑/删除。 */
export function ObjectDetailFooter({
  t,
  guidePages,
  onHistory,
  onAttachments,
  onEdit,
  onDelete,
}: {
  t: T;
  guidePages: GuidePage[];
  onHistory: () => void;
  onAttachments: () => void;
  onEdit?: () => void;
  onDelete: () => void;
}) {
  return (
    <div className={styles.modalFooter}>
      <div className={styles.guideWrapper}>
        <PageGuideButton pages={guidePages} />
      </div>
      <div className={styles.footerActions}>
        <button onClick={onHistory} className={`${styles.actionBtn} ${styles.footerBtn}`}>
          <Clock size={ICON_SIZE.sm} />
          <span className={styles.actionLabel}>{t('common:history')}</span>
        </button>
        <button onClick={onAttachments} className={`${styles.actionBtn} ${styles.footerBtn}`}>
          <Paperclip size={ICON_SIZE.sm} />
          <span className={styles.actionLabel}>{t('common:attachments')}</span>
        </button>
        {onEdit && (
          <button onClick={onEdit} className={`${styles.actionBtn} ${styles.footerBtn}`}>
            <Pencil size={ICON_SIZE.sm} />
            <span className={styles.actionLabel}>{t('common:edit')}</span>
          </button>
        )}
        <div className={styles.deleteBtnWrapper}>
          <DeleteButton onClick={onDelete} title={t('common:delete')}>
            <span className={styles.actionLabel}>{t('common:delete')}</span>
          </DeleteButton>
        </div>
      </div>
    </div>
  );
}
