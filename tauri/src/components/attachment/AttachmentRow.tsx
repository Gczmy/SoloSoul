import { memo, useRef, useState } from 'react';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { AttachmentFileNameBlock } from './AttachmentFileNameBlock';
import { AttachmentActions } from './AttachmentActions';
import { AttachmentTypeIcon, AttachmentExtBadge } from './AttachmentFormatBadge';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import type { AttachmentMeta } from './attachmentManagerTypes';

/** 移动端操作按钮行间距：原 4px 的 80%——窄屏让宽，防六个按钮溢出卡片（软删除不可见）。 */
const MOBILE_ACTIONS_GAP = 4 * 0.8;

interface AttachmentRowProps {
  item: AttachmentMeta;
  objectId: string;
  showTrash: boolean;
  isChecked: boolean;
  isRenaming: boolean;
  onToggleSelect: (compositeKey: string) => void;
  onRenameConfirm: (newName: string) => void;
  onRenameCancel: () => void;
  onPreview: (item: AttachmentMeta) => void;
  onStartRename: (item: AttachmentMeta, objectId: string) => void;
  onDownload: (item: AttachmentMeta) => void;
  onShare: (item: AttachmentMeta) => void;
  /** 编辑描述与标签（附件行级入口） */
  onEditMeta?: (item: AttachmentMeta, objectId: string) => void;
  onSoftDelete: (item: AttachmentMeta, objectId: string) => void;
  onRestore: (item: AttachmentMeta, objectId: string) => void;
  onPermanentDelete: (item: AttachmentMeta, objectId: string) => void;
}

/**
 * 自包含的重命名输入框（P217）。
 *
 * 输入值作为组件本地 state，由行内自行管理——击键只重渲染当前行，
 * 不再逐字驱动顶层 `GlobalAttachmentManager` 整树重建。仅在 `isRenaming`
 * 为真时挂载（autoFocus 聚焦），确认/取消后随 isRenaming 翻转卸载。
 */
function RenameInput({
  initialName,
  onConfirm,
  onCancel,
}: {
  initialName: string;
  onConfirm: (newName: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(initialName);
  // 防 Enter→blur 双触发重复提交
  const submittedRef = useRef(false);

  const confirm = () => {
    if (submittedRef.current) return;
    submittedRef.current = true;
    onConfirm(value);
  };

  const cancel = () => {
    submittedRef.current = true;
    onCancel();
  };

  return (
    <input
      autoFocus
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') confirm();
        if (e.key === 'Escape') cancel();
      }}
      onBlur={confirm}
      style={{
        flex: 1,
        minWidth: 0,
        padding: '2px 6px',
        fontSize: 'var(--text-sm)',
        borderRadius: 4,
        border: '1px solid var(--accent-primary)',
        background: 'transparent',
        color: 'var(--text-primary)',
        outline: 'none',
      }}
    />
  );
}

/** 附件单行（移动端多行布局 / 桌面端单行布局）。 */
function AttachmentRowBase({
  item,
  objectId,
  showTrash,
  isChecked,
  isRenaming,
  onToggleSelect,
  onRenameConfirm,
  onRenameCancel,
  onPreview,
  onStartRename,
  onDownload,
  onShare,
  onEditMeta,
  onSoftDelete,
  onRestore,
  onPermanentDelete,
}: AttachmentRowProps) {
  const compositeKey = `${objectId}::${item.id}`;
  const isMobile = isMobilePlatformSync();

  const renameInput = (
    <RenameInput
      initialName={item.fileName}
      onConfirm={onRenameConfirm}
      onCancel={onRenameCancel}
    />
  );

  const actions = (
    <AttachmentActions
      showTrash={showTrash}
      onPreview={() => onPreview(item)}
      onStartRename={() => onStartRename(item, objectId)}
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

          {isRenaming ? (
            renameInput
          ) : (
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
          )}
        </div>
        {/* 第2行：操作按钮——0 缩进（移出内容列，与勾选框同左缘对齐，不再占
            勾选框宽度的缩进空间）；间距 4px → 3.2px（80%），窄屏下六个按钮
            （含软删除）可全部落在卡片范围内。重命名进行时不渲染。 */}
        {!isRenaming && <div style={{ display: 'flex', gap: MOBILE_ACTIONS_GAP }}>{actions}</div>}
      </div>
    );
  }

  // 桌面端：单行布局
  return (
    <div
      key={item.id}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        padding: '6px 8px 6px 40px',
        fontSize: 'var(--text-sm)',
        borderBottom: '1px solid var(--border-subtle)',
      }}
    >
      <SelectCheckbox
        checked={isChecked}
        onClick={(e) => {
          e.stopPropagation();
          onToggleSelect(compositeKey);
        }}
      />
      <AttachmentTypeIcon
        item={item}
        size={ICON_SIZE.sm}
        style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
      />
      <AttachmentExtBadge fileName={item.fileName} />

      {isRenaming ? (
        renameInput
      ) : (
        <AttachmentFileNameBlock
          fileName={item.fileName}
          sizeBytes={item.sizeBytes}
          createdAt={item.createdAt}
          showTrash={showTrash}
          description={item.description}
          tags={item.tags}
        />
      )}

      <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
        {showTrash || !isRenaming ? actions : null}
      </div>
    </div>
  );
}

/**
 * P217：memo 化——比较器只比较数据 props（item/objectId/showTrash/isChecked/isRenaming），
 * 忽略全部回调身份。安全性依据：① 回调要么接收显式参数（item/objectId/新文件名），
 * 要么使用函数式 setState，持旧引用无害；② 唯一闭包捕获 hook 状态的是
 * onRenameConfirm（依赖 renamingId/renameObjectId），但其正确性由 isRenaming 保证——
 * renamingId 一旦变化，目标行 isRenaming 必翻转并重渲染拿到新闭包，非目标行不会调用。
 */
function attachmentRowPropsEqual(prev: AttachmentRowProps, next: AttachmentRowProps): boolean {
  return (
    prev.item === next.item &&
    prev.objectId === next.objectId &&
    prev.showTrash === next.showTrash &&
    prev.isChecked === next.isChecked &&
    prev.isRenaming === next.isRenaming
  );
}

export const AttachmentRow = memo(AttachmentRowBase, attachmentRowPropsEqual);
