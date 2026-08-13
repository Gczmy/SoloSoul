// ============================================================
// TrashDetailSections.tsx — 回收站详情面板的纯展示子组件
// P224-① 拆分自 TrashDetailPanel.tsx 的 ObjectDetailContent：
// 6 个渲染区块抽为独立纯展示组件（props 透传、零行为变更）。
// 与 TrashSnapshotView.tsx（快照展示域）相互独立，无循环依赖。
// ============================================================

import { useTranslation } from 'react-i18next';
import { ArrowLeft, X, RotateCcw } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { ValueContainer } from '@/components/ui/ValueContainer';
import { ICON_SIZE } from '@/lib/constants';
import { resolveCollectionLabel } from '@/lib/utils';
import { AttachmentFileNameBlock } from '@/components/attachment/AttachmentFileNameBlock';
import {
  AttachmentTypeIcon,
  AttachmentExtBadge,
} from '@/components/attachment/AttachmentFormatBadge';
import type { CustomPage } from '@/stores/settingsStore';
import type { PropertyType, SensitivityLevel, UserTemplate } from '@/types/template';
import type { TrashDetail, TrashAttachment } from './types';
import { SnapshotContent } from './TrashSnapshotView';

export function TrashDetailHeader({
  item,
  onClose,
  showBackButton,
  onBack,
}: {
  item: TrashDetail;
  onClose: () => void;
  showBackButton?: boolean;
  onBack?: () => void;
}) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <>
      {showBackButton && onBack && (
        <button
          onClick={onBack}
          className="interactive-icon"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            border: 'none',
            borderRadius: 6,
            cursor: 'pointer',
            padding: '4px 8px',
            fontSize: 'var(--text-body-sm)',
            fontFamily: 'inherit',
            marginBottom: 8,
          }}
        >
          <ArrowLeft size={ICON_SIZE.sm} />
          {t('common:back', { defaultValue: 'Back' })}
        </button>
      )}

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          marginBottom: 16,
        }}
      >
        <div>
          <h3
            style={{
              fontSize: 'var(--text-section-title)',
              fontWeight: 600,
              margin: 0,
              overflowWrap: 'break-word',
              wordBreak: 'break-word',
            }}
          >
            {item.name}
          </h3>
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {t(`settings:trash_type_${item.itemType}`)}
          </span>
        </div>
        <button
          onClick={onClose}
          className="interactive-accent"
          style={{
            border: 'none',
            borderRadius: 6,
            cursor: 'pointer',
            padding: 4,
          }}
        >
          <X size={ICON_SIZE.lg} />
        </button>
      </div>
    </>
  );
}

export function TrashMetaInfo({
  item,
  customPages,
}: {
  item: TrashDetail;
  customPages: CustomPage[];
}) {
  const { t } = useTranslation(['settings', 'navigation']);
  return (
    <>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
          fontSize: 'var(--text-body-sm)',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:delete_time')}</span>
          <span>{new Date(item.deletedAt).toLocaleString()}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:original_location')}</span>
          <span>
            {item.sectionType === 'page'
              ? t('settings:log.entity.page')
              : resolveCollectionLabel(item.sectionType || item.originalLocation, customPages, t)}
          </span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:remaining_retention')}</span>
          <span>
            {item.remainingDays != null
              ? t('settings:trash_expires_in', { days: item.remainingDays })
              : t('settings:never_delete')}
          </span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:deleted_by')}</span>
          <span>
            {item.deletedBy === 'user'
              ? t('settings:deleted_by_user')
              : t('settings:deleted_by_system')}
          </span>
        </div>
      </div>
    </>
  );
}

export function TrashFieldList({ item }: { item: TrashDetail }) {
  const { t } = useTranslation(['settings', 'common', 'editor']);
  if (item.previewProperties.length === 0) {
    return null;
  }

  return (
    <>
      <div
        style={{
          marginTop: 16,
          borderTop: '1px solid var(--border-subtle)',
          paddingTop: 12,
        }}
      >
        <h4 style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600, marginBottom: 8 }}>
          {t('settings:content_preview')}
        </h4>
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 6,
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
          }}
        >
          {item.previewProperties.map((p, i) => {
            const isTemplate = item.itemType === 'template';
            const propType = (p as Record<string, unknown>).type as PropertyType | undefined;
            const explicitSensitivity = (p as Record<string, unknown>).sensitivityLevel as
              | SensitivityLevel
              | undefined;
            const fieldId = (p as Record<string, unknown>).fieldId as string | undefined;
            const fallbackSensitivity = fieldId ? item.propertyLabels?.[fieldId] : undefined;
            const sensitivity = explicitSensitivity || fallbackSensitivity;
            const displayKey =
              p.key === '__dynamic_group__'
                ? t('editor:field_types.dynamic_group', p.key)
                : p.key === 'description'
                  ? t('common:description')
                  : p.key;

            const formatTypeLabel = (type?: PropertyType) =>
              type ? t(`editor:field_types.${type}`, type) : '';

            if (propType === 'dynamic_group') {
              const rawValue = (p as Record<string, unknown>).value;
              let arr: unknown[] | undefined;
              if (Array.isArray(rawValue)) {
                arr = rawValue;
              } else if (typeof rawValue === 'string') {
                try {
                  const parsed = JSON.parse(rawValue);
                  if (Array.isArray(parsed)) arr = parsed;
                } catch {
                  arr = undefined;
                }
              }
              const childItems =
                arr
                  ?.filter(
                    (child): child is Record<string, unknown> =>
                      child !== null && typeof child === 'object',
                  )
                  .map((child) => ({
                    name: typeof child.name === 'string' ? child.name : String(child.id || ''),
                    value:
                      typeof child.value === 'string'
                        ? child.value
                        : child.value !== undefined && child.value !== null
                          ? JSON.stringify(child.value)
                          : '',
                    type: typeof child.type === 'string' ? (child.type as PropertyType) : undefined,
                  })) ?? [];

              return (
                <div key={i} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                  {/* Parent dynamic group row */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <FieldTypeIcon type="dynamic_group" size={ICON_SIZE.sm} />
                    <span style={{ fontWeight: 500, flexShrink: 0 }}>{displayKey}</span>
                    {sensitivity && <SensitivityBadge level={sensitivity} />}
                  </div>
                  {/* Child fields */}
                  {childItems.map((child) => {
                    const childValue = isTemplate ? formatTypeLabel(child.type) : child.value;
                    return (
                      <div
                        key={child.name}
                        style={{
                          display: 'flex',
                          flexWrap: 'wrap',
                          alignItems: 'flex-start',
                          gap: 8,
                          marginLeft: 16,
                          fontSize: 'var(--text-caption)',
                          color: 'var(--text-secondary)',
                        }}
                      >
                        <div
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 6,
                            flex: '0 0 auto',
                          }}
                        >
                          {child.type && <FieldTypeIcon type={child.type} size={ICON_SIZE.sm} />}
                          <span style={{ fontWeight: 500, flexShrink: 0 }}>{child.name}</span>
                        </div>
                        <ValueContainer value={childValue}>
                          <span style={{ color: 'var(--text-tertiary)' }}>{childValue}</span>
                        </ValueContainer>
                      </div>
                    );
                  })}
                </div>
              );
            }

            const displayValue = isTemplate
              ? formatTypeLabel(propType)
              : typeof p.value === 'string'
                ? p.value
                : p.value !== undefined && p.value !== null
                  ? JSON.stringify(p.value)
                  : '';
            return (
              <div
                key={i}
                style={{
                  display: 'flex',
                  flexWrap: 'wrap',
                  alignItems: 'flex-start',
                  gap: 8,
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, flex: '0 0 auto' }}>
                  {propType && <FieldTypeIcon type={propType} size={ICON_SIZE.sm} />}
                  <span style={{ fontWeight: 500, flexShrink: 0 }}>{displayKey}</span>
                  {sensitivity && <SensitivityBadge level={sensitivity} />}
                </div>
                <ValueContainer value={displayValue}>
                  <span style={{ color: 'var(--text-tertiary)' }}>{displayValue}</span>
                </ValueContainer>
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}

export function TrashAttachmentsSection({
  activeAttachments,
  deletedAttachments,
  expanded,
  showTrash,
  onToggle,
  onSetShowTrash,
}: {
  activeAttachments: TrashAttachment[];
  deletedAttachments: TrashAttachment[];
  expanded: boolean;
  showTrash: boolean;
  onToggle: () => void;
  onSetShowTrash: (show: boolean) => void;
}) {
  const { t } = useTranslation(['common']);
  return (
    <>
      <div
        style={{
          marginTop: 12,
          borderTop: '1px solid var(--border-subtle)',
          paddingTop: 10,
        }}
      >
        <div
          onClick={onToggle}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            cursor: 'pointer',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 600,
            userSelect: 'none',
          }}
        >
          <span
            style={{
              transform: expanded ? 'rotate(90deg)' : 'none',
              transition: 'transform 0.15s',
              fontSize: 'var(--text-badge)',
            }}
          >
            ▶
          </span>
          {t('common:attachments')} ({activeAttachments.length + deletedAttachments.length})
        </div>
        {expanded && (
          <div style={{ marginTop: 8 }}>
            {deletedAttachments.length > 0 && (
              <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
                <button
                  onClick={() => onSetShowTrash(false)}
                  className={showTrash ? 'interactive-toolbar' : 'selected-accent'}
                  style={{
                    padding: '4px 10px',
                    borderRadius: 6,
                    fontSize: 'var(--text-badge)',
                    fontWeight: 500,
                    borderWidth: 1,
                    borderStyle: 'solid',
                    color: showTrash ? 'var(--text-primary)' : 'var(--accent-primary)',
                    boxShadow: showTrash ? 'none' : '0 0 0 1px var(--accent-primary)',
                    cursor: 'pointer',
                    transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                  }}
                >
                  {t('common:active')} ({activeAttachments.length})
                </button>
                <button
                  onClick={() => onSetShowTrash(true)}
                  className={showTrash ? 'selected-danger' : 'interactive-danger'}
                  style={{
                    padding: '4px 10px',
                    borderRadius: 6,
                    fontSize: 'var(--text-badge)',
                    fontWeight: 500,
                    borderWidth: 1,
                    borderStyle: 'solid',
                    color: showTrash ? '#e74c3c' : 'var(--text-primary)',
                    cursor: 'pointer',
                    transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                  }}
                >
                  {t('common:trash')} ({deletedAttachments.length})
                </button>
              </div>
            )}
            {(showTrash ? deletedAttachments : activeAttachments).length === 0 ? (
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  padding: '8px 0',
                }}
              >
                {t('common:no_data')}
              </p>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                {(showTrash ? deletedAttachments : activeAttachments).map((a) => {
                  return (
                    <div
                      key={a.id}
                      className="interactive-row"
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        fontSize: 'var(--text-caption)',
                        padding: '8px 10px',
                        borderRadius: 6,
                        borderWidth: 1,
                        borderStyle: 'solid',
                        cursor: 'default',
                      }}
                    >
                      <AttachmentTypeIcon
                        item={a}
                        size={ICON_SIZE.md}
                        style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
                      />
                      <AttachmentFileNameBlock
                        fileName={a.fileName}
                        sizeBytes={a.sizeBytes}
                        createdAt={a.createdAt}
                        showTrash={showTrash}
                        // 与 AttachmentListItem（对象详情卡）同款紧凑元信息行字号，保持回收站详情原视觉
                        metaStyle={{ fontSize: 'var(--text-badge)' }}
                        description={a.description}
                        tags={a.tags}
                      />
                      <AttachmentExtBadge fileName={a.fileName} />
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}
      </div>
    </>
  );
}

export function SnapshotSummaryRow({
  item,
  expanded,
  onToggle,
  currentSnapIdx,
  data,
  loading,
  detailTemplate,
  onChangeSnapshot,
}: {
  item: TrashDetail;
  expanded: boolean;
  onToggle: () => void;
  currentSnapIdx: number;
  data: Record<string, unknown> | null | undefined;
  loading: boolean | undefined;
  detailTemplate: UserTemplate | null;
  onChangeSnapshot: (newIdx: number) => void;
}) {
  const { t } = useTranslation(['settings']);
  return (
    <>
      <div
        style={{
          marginTop: 12,
          borderTop: '1px solid var(--border-subtle)',
          paddingTop: 10,
        }}
      >
        <div
          onClick={onToggle}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            cursor: 'pointer',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 600,
            userSelect: 'none',
            ...(expanded
              ? {}
              : {
                  borderBottom: '1px solid var(--border-subtle)',
                  paddingBottom: 10,
                  marginBottom: 10,
                }),
          }}
        >
          <span
            style={{
              transform: expanded ? 'rotate(90deg)' : 'none',
              transition: 'transform 0.15s',
              fontSize: 'var(--text-badge)',
            }}
          >
            ▶
          </span>
          {t('settings:data_snapshots')} ({item.snapshots.length})
        </div>
        {expanded && (
          <SnapshotContent
            _detailId={item.id}
            snapshots={item.snapshots}
            currentSnapIdx={currentSnapIdx}
            data={data}
            loading={loading}
            detailTemplate={detailTemplate}
            currentPropertyLabels={item.propertyLabels}
            onChangeSnapshot={onChangeSnapshot}
          />
        )}
      </div>
    </>
  );
}

export function TrashDetailActions({
  onRestore,
  onDelete,
}: {
  onRestore: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation(['common']);
  return (
    <>
      <div style={{ marginTop: 16, display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
        <Button size="sm" variant="tertiary" onClick={onRestore}>
          <RotateCcw size={ICON_SIZE.xs} style={{ marginRight: 4 }} /> {t('common:restore')}
        </Button>
        <DeleteButton onClick={onDelete} title={t('common:delete_permanently')}>
          {t('common:delete_permanently')}
        </DeleteButton>
      </div>
    </>
  );
}
