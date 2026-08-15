import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { AttachmentRow } from './AttachmentRow';
import { isMobilePlatformSync } from '@/lib/platform';
import type { AttachmentMeta } from './attachmentManagerTypes';

vi.mock('@/lib/platform', () => ({
  isMobilePlatformSync: vi.fn(() => false),
}));

const item: AttachmentMeta = {
  id: 'att_1',
  objectId: 'obj_1',
  fileName: 'report.pdf',
  mimeType: 'application/pdf',
  sizeBytes: 1024,
  createdAt: '2026-07-01T00:00:00Z',
};

function setupRow(props: Partial<Parameters<typeof AttachmentRow>[0]> = {}) {
  const onToggleSelect = vi.fn();
  const onPreview = vi.fn();
  const onDownload = vi.fn();
  const onShare = vi.fn();
  const onEditMeta = vi.fn();
  const onSoftDelete = vi.fn();
  const onRestore = vi.fn();
  const onPermanentDelete = vi.fn();

  render(
    <AttachmentRow
      item={item}
      objectId="obj_1"
      showTrash={false}
      isChecked={false}
      onToggleSelect={onToggleSelect}
      onPreview={onPreview}
      onDownload={onDownload}
      onShare={onShare}
      onEditMeta={onEditMeta}
      onSoftDelete={onSoftDelete}
      onRestore={onRestore}
      onPermanentDelete={onPermanentDelete}
      {...props}
    />,
  );

  return {
    onToggleSelect,
    onPreview,
    onDownload,
    onShare,
    onEditMeta,
    onSoftDelete,
    onRestore,
    onPermanentDelete,
  };
}

describe('AttachmentRow', () => {
  it('renders file name and meta info', () => {
    setupRow();
    expect(screen.getByText('report.pdf')).toBeInTheDocument();
    expect(screen.getByText(/1\.0 KB/)).toBeInTheDocument();
  });

  it('exposes action buttons (preview/download/share/edit-attributes/delete)', () => {
    setupRow();
    // 重命名与编辑描述和标签已合并：常规态 5 个操作按钮（预览/下载/转发/编辑属性/删除）
    expect(screen.getAllByRole('button').length).toBeGreaterThanOrEqual(5);
  });

  it('renders share button and fires onShare with the item', () => {
    const { onShare } = setupRow();
    const shareButton = screen.getByTitle('common:forward');
    expect(shareButton).toBeInTheDocument();
    fireEvent.click(shareButton);
    expect(onShare).toHaveBeenCalledTimes(1);
    expect(onShare).toHaveBeenCalledWith(item);
  });

  it('fires onEditMeta with the item and objectId (合并后的编辑附件属性入口)', () => {
    const { onEditMeta } = setupRow();
    // 单测无 i18n 实例：t() 带 defaultValue 时返回 defaultValue（'Edit Attachment Attributes'）
    const editBtn = screen.getByTitle('Edit Attachment Attributes');
    expect(editBtn).toBeInTheDocument();
    fireEvent.click(editBtn);
    expect(onEditMeta).toHaveBeenCalledTimes(1);
    expect(onEditMeta).toHaveBeenCalledWith(item, 'obj_1');
  });

  it('移动端：操作按钮行 0 缩进（移出内容列）且间距为 80%（3.2px）', () => {
    vi.mocked(isMobilePlatformSync).mockReturnValue(true);
    setupRow();

    const deleteBtn = screen.getByTitle('common:delete');
    // BadgeIconButton 结构：div.wrapper > button
    const wrapper = deleteBtn.parentElement!;
    const actionsRow = wrapper.parentElement!;
    expect(actionsRow.style.display).toBe('flex');
    expect(actionsRow.style.gap).toBe('3.2px');
    // 0 缩进：按钮行是行容器（flex column）的直接子元素，
    // 而非缩进在「勾选框内容列（flex:1）」之内
    const rowContainer = actionsRow.parentElement!;
    expect(rowContainer.style.flexDirection).toBe('column');
  });

  it('移动端：附件图标位于元信息行左侧（与名称列对齐）而非勾选框下方', () => {
    vi.mocked(isMobilePlatformSync).mockReturnValue(true);
    setupRow();
    // 元信息文本（span）所在 flex 行即「大小·时间」行；其首个子元素应为图标 SVG
    // → 图标缩进到与名称同列、与元信息同行（方案B）；若图标仍竖排在勾选框下方
    // （旧布局），元信息行内将无任何 SVG，本断言即失败。
    const metaText = screen.getByText(/1\.0 KB/);
    const metaRow = metaText.parentElement;
    expect(metaRow).not.toBeNull();
    expect(metaRow!.firstElementChild?.tagName.toLowerCase()).toBe('svg');
    // 格式名称徽章与图标同在元信息行内（[图标][PDF]大小·时间）
    expect(metaRow!.textContent).toContain('PDF');
  });

  it('桌面端：第1行勾选框+附件名称，第2行[图标][徽章]附件信息', () => {
    vi.mocked(isMobilePlatformSync).mockReturnValue(false);
    setupRow();
    // 元信息行首个子元素为图标 SVG，随后是格式徽章——图标/徽章不再占据名称行首部
    const metaText = screen.getByText(/1\.0 KB/);
    const metaRow = metaText.parentElement;
    expect(metaRow).not.toBeNull();
    expect(metaRow!.firstElementChild?.tagName.toLowerCase()).toBe('svg');
    expect(metaRow!.textContent).toContain('PDF');
    // 勾选框与名称同列（flex-start 对齐）：勾选框不在元信息行内
    expect(metaRow!.querySelector('input[type=checkbox]')).toBeNull();
  });

  it('显示格式名称徽章（随扩展名变化：report.pdf → PDF）', () => {
    setupRow();
    expect(screen.getByText('PDF')).toBeInTheDocument();
  });

  it('图片附件显示图片格式图标与对应徽章（photo.png → PNG）', () => {
    setupRow({
      item: {
        ...item,
        id: 'att_2',
        fileName: 'photo.png',
        mimeType: 'image/png',
      },
    });
    expect(screen.getByText('PNG')).toBeInTheDocument();
    // 徽章紧跟在格式图标之后；图标必须是 Image（lucide-image）而非统一回形针
    const badge = screen.getByText('PNG');
    expect(badge.tagName.toLowerCase()).toBe('span');
    const iconSvg = badge.previousElementSibling;
    expect(iconSvg?.tagName.toLowerCase()).toBe('svg');
    expect((iconSvg as HTMLElement).classList.contains('lucide-image')).toBe(true);
  });
});
