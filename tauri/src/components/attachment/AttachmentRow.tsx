import { memo } from 'react';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { AttachmentFileNameBlock } from './AttachmentFileNameBlock';
import { AttachmentActions } from './AttachmentActions';
import { AttachmentTypeIcon, AttachmentExtBadge } from './AttachmentFormatBadge';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import type { AttachmentMeta } from './attachmentManagerTypes';

/** 移动端操作按钮行间距：原 4px 的 80%——窄屏让宽，防按钮溢出卡片（软删除不可见）。 */
const MOBILE_ACTIONS_GAP = 4 * 0.8;

interface AttachmentRowProps {
  item: AttachmentMeta;
  objectId: string;
  showTrash: boolean;
  isChecked: boolean;
  onToggleSelect: (compositeKey: string) => void;
  onPreview: (item: AttachmentMeta) => void;
  onDownload: (item: AttachmentMeta) => void;
  onShare: (item: AttachmentMeta) => void;
  /** 编辑附件属性（名称/描述/标签） */
  onEditMeta?: (item: AttachmentMeta, objectId: string) => void;
  onSoftDelete: (item: AttachmentMeta, objectId: string) => void;
  onRestore: (item: AttachmentMeta, objectId: string) => void;
  onPermanentDelete: (item: AttachmentMeta, objectId: string) => void;
}

/** 附件单行（移动端多行布局 / 桌面端单行布局）。 */
function AttachmentRowBase({
  item,
  objectId,
  showTrash,
  isChecked,
  onToggleSelect,
  onPreview,
  onDownload,
  onShare,
  onEditMeta,
  onSoftDelete,
  onRestore,
  onPermanentDelete,
}: AttachmentRowProps) {
  const compositeKey = `${objectId}::${item.id}`;
  const isMobile = isMobilePlatformSync();

  const actions = (
    <AttachmentActions
      showTrash={showTrash}
      onPreview={() => onPreview(item)}
      onDownload={() => onDownload(item)}
      onShare={() => onShare(item)}
      onEditMeta={() => onEditMeta?.(item, objectId)}
      onSoftDelete={() => onSoftDelete(item, objectId)}
      onRestore={() => onRestore(item, objectId)}
      onPermanentDelete={() => onPermanentDelete(item, objectId)}
    />
  );

  // 移动端：多行布局 — 勾选框独占左上角；内容列（勾选框宽度的天然缩进）第1行 文件名，
  // 第2行 [格式图标][格式名称徽章]大小·时间（图标+徽章经 metaLeadingIcon 移入元信息行
  // 左侧，与名称列对齐），第3行 操作按钮；无层级缩进与页面/对象行对齐
  if (isMobile) {
    return (
      <div
        key={item.id}
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 6,
          padding: '6px 8px 6px 14px',
          fontSize: 'var(--text-sm)',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        {/* 第1行：勾选框 + 内容列（文件名 / 元信息） */}
        <div style={{ display: 'flex', gap: 6, alignItems: 'flex-start' }}>
          <div style={{ display: 'flex', flexDirection: 'column', flexShrink: 0 }}>
            <SelectCheckbox
              checked={isChecked}
              onClick={(e) => {
                e.stopPropagation();
                onToggleSelect(compositeKey);
              }}
            />
          </div>

          <div style={{ flex: 1, minWidth: 0 }}>
            <AttachmentFileNameBlock
              fileName={item.fileName}
              sizeBytes={item.sizeBytes}
              createdAt={item.createdAt}
              showTrash={showTrash}
              description={item.description}
              tags={item.tags}
              metaLeadingIcon={
                <>
                  <AttachmentTypeIcon
                    item={item}
                    size={ICON_SIZE.sm}
                    style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
                  />
                  <AttachmentExtBadge fileName={item.fileName} />
                </>
              }
            />
          </div>
        </div>
        {/* 第2行：操作按钮——0 缩进（移出内容列，与勾选框同左缘对齐），
            间距 4px → 3.2px（80%），窄屏下按钮（含软删除）可全部落在卡片范围内。 */}
        <div style={{ display: 'flex', gap: MOBILE_ACTIONS_GAP }}>{actions}</div>
      </div>
    );
  }

  // 桌面端：两行布局 — 第1行 [勾选框] 附件名称（勾选框与名称行对齐）；
  // 第2行 [格式图标][格式徽章] 附件信息（图标+徽章经 metaLeadingIcon 移入元信息行左侧，
  // 不再占据名称行首部）；操作按钮居右垂直居中。
  // 勾选框垂直对齐：名称行行高（14px 字号 × 1.4 ≈ 19.6px）大于勾选框自身 14px——
  // 容器 alignSelf 对齐的是整块（名称+元信息），故用定高容器 + 内部垂直居中，
  // 使勾选框与名称文本行高中心对齐。行容器显式声明 lineHeight: 1.4，勾选框
  // 容器高度用同一度量（text-sm × 1.4）推导，两者始终一致、不依赖字体默认值。
  return (
    <div
      key={item.id}
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        gap: 6,
        padding: '6px 8px 6px 40px',
        fontSize: 'var(--text-sm)',
        lineHeight: 1.4,
        borderBottom: '1px solid var(--border-subtle)',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          height: 'calc(var(--text-sm) * 1.4)',
          flexShrink: 0,
        }}
      >
        <SelectCheckbox
          checked={isChecked}
          onClick={(e) => {
            e.stopPropagation();
            onToggleSelect(compositeKey);
          }}
        />
      </div>

      <AttachmentFileNameBlock
        fileName={item.fileName}
        sizeBytes={item.sizeBytes}
        createdAt={item.createdAt}
        showTrash={showTrash}
        description={item.description}
        tags={item.tags}
        metaLeadingIcon={
          <>
            <AttachmentTypeIcon
              item={item}
              size={ICON_SIZE.sm}
              style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
            />
            <AttachmentExtBadge fileName={item.fileName} />
          </>
        }
      />

      <div style={{ display: 'flex', gap: 4, flexShrink: 0, alignSelf: 'center' }}>{actions}</div>
    </div>
  );
}

/**
 * P217：memo 化——比较器只比较数据 props（item/objectId/showTrash/isChecked），
 * 忽略全部回调身份。安全性依据：回调要么接收显式参数（item/objectId），
 * 要么使用函数式 setState，持旧引用无害。
 */
function attachmentRowPropsEqual(prev: AttachmentRowProps, next: AttachmentRowProps): boolean {
  return (
    prev.item === next.item &&
    prev.objectId === next.objectId &&
    prev.showTrash === next.showTrash &&
    prev.isChecked === next.isChecked
  );
}

export const AttachmentRow = memo(AttachmentRowBase, attachmentRowPropsEqual);
