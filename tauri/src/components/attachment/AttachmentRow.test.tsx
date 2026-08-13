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
  const onRenameConfirm = vi.fn();
  const onRenameCancel = vi.fn();
  const onPreview = vi.fn();
  const onStartRename = vi.fn();
  const onDownload = vi.fn();
  const onShare = vi.fn();
  const onSoftDelete = vi.fn();
  const onRestore = vi.fn();
  const onPermanentDelete = vi.fn();

  render(
    <AttachmentRow
      item={item}
      objectId="obj_1"
      showTrash={false}
      isChecked={false}
      isRenaming={false}
      onToggleSelect={onToggleSelect}
      onRenameConfirm={onRenameConfirm}
      onRenameCancel={onRenameCancel}
      onPreview={onPreview}
      onStartRename={onStartRename}
      onDownload={onDownload}
      onShare={onShare}
      onSoftDelete={onSoftDelete}
      onRestore={onRestore}
      onPermanentDelete={onPermanentDelete}
      {...props}
    />,
  );

  return {
    onToggleSelect,
    onRenameConfirm,
    onRenameCancel,
    onPreview,
    onStartRename,
    onDownload,
    onShare,
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

  it('shows rename input pre-filled when isRenaming', () => {
    setupRow({ isRenaming: true });
    const input = screen.getByDisplayValue('report.pdf');
    expect(input).toBeInTheDocument();
  });

  it('P217: typing only updates local input, does not submit per keystroke', () => {
    const { onRenameConfirm } = setupRow({ isRenaming: true });
    const input = screen.getByDisplayValue('report.pdf');
    fireEvent.change(input, { target: { value: 'new-name.pdf' } });
    // 本地输入态：任何一次击键都不应触发 confirm
    expect(onRenameConfirm).not.toHaveBeenCalled();
    // 输入框随本地 state 更新
    expect(screen.getByDisplayValue('new-name.pdf')).toBeInTheDocument();
  });

  it('P217: Enter submits the typed value once', () => {
    const { onRenameConfirm } = setupRow({ isRenaming: true });
    const input = screen.getByDisplayValue('report.pdf');
    fireEvent.change(input, { target: { value: 'new-name.pdf' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onRenameConfirm).toHaveBeenCalledTimes(1);
    expect(onRenameConfirm).toHaveBeenCalledWith('new-name.pdf');
  });

  it('P217: Enter→blur does not double-submit', () => {
    const { onRenameConfirm } = setupRow({ isRenaming: true });
    const input = screen.getByDisplayValue('report.pdf');
    fireEvent.change(input, { target: { value: 'new-name.pdf' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    fireEvent.blur(input);
    expect(onRenameConfirm).toHaveBeenCalledTimes(1);
  });

  it('P217: Escape cancels without submitting', () => {
    const { onRenameConfirm, onRenameCancel } = setupRow({ isRenaming: true });
    const input = screen.getByDisplayValue('report.pdf');
    fireEvent.change(input, { target: { value: 'new-name.pdf' } });
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(onRenameCancel).toHaveBeenCalledTimes(1);
    expect(onRenameConfirm).not.toHaveBeenCalled();
  });

  it('blur confirms the current value', () => {
    const { onRenameConfirm } = setupRow({ isRenaming: true });
    const input = screen.getByDisplayValue('report.pdf');
    fireEvent.change(input, { target: { value: 'blurred.pdf' } });
    fireEvent.blur(input);
    expect(onRenameConfirm).toHaveBeenCalledTimes(1);
    expect(onRenameConfirm).toHaveBeenCalledWith('blurred.pdf');
  });

  it('hides rename input when not renaming and exposes actions', () => {
    setupRow();
    expect(screen.queryByDisplayValue('report.pdf')).not.toBeInTheDocument();
    // 非回收站视图：预览/重命名/下载/转发/删除五个操作按钮
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

  it('hides share button when renaming', () => {
    setupRow({ isRenaming: true });
    expect(screen.queryByTitle('common:forward')).not.toBeInTheDocument();
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
