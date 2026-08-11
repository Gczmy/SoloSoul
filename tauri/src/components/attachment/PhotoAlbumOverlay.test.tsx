import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { PhotoAlbumOverlay } from './PhotoAlbumOverlay';
import type { AttachmentItem } from '@/lib/attachmentUtils';

const mockInvoke = vi.mocked(invoke);

function makeItem(id: string): AttachmentItem {
  return {
    id,
    objectId: 'obj-1',
    fileName: `${id}.png`,
    mimeType: 'image/png',
    sizeBytes: 100,
    createdAt: '2024-01-01T00:00:00Z',
    vaultPath: `/vault/attachments/obj-1/${id}.png`,
    srcPath: null,
  };
}

describe('PhotoAlbumOverlay', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('opens the viewer on cell click and returns to grid on back', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(<PhotoAlbumOverlay items={[makeItem('a'), makeItem('b')]} onClose={vi.fn()} />);

    const grid = await screen.findByTestId('photo-album-grid');
    const cells = within(grid).getAllByRole('button');
    fireEvent.click(cells[0]);

    await waitFor(() => {
      expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
    });

    fireEvent.click(screen.getByRole('button', { name: /common:back_to_album/i }));
    await waitFor(() => {
      expect(screen.queryByTestId('photo-viewer-counter')).not.toBeInTheDocument();
    });
  });

  it('closes via header close button', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    render(<PhotoAlbumOverlay items={[makeItem('a')]} onClose={onClose} />);

    // 等待缩略图异步加载完成，避免测试结束时状态更新产生 act() 警告
    await screen.findByAltText('a.png');
    fireEvent.click(screen.getByRole('button', { name: /common:close/i }));
    expect(onClose).toHaveBeenCalled();
  });

  it('filters photos by tag chip (需求4：标签分区筛选)', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const tagged = makeItem('tagged');
    const plain = makeItem('plain');
    tagged.tags = ['vacation'];
    render(<PhotoAlbumOverlay items={[tagged, plain]} onClose={vi.fn()} />);

    // 网格中应有两个缩略图
    await screen.findByAltText('tagged.png');
    expect(screen.getByAltText('plain.png')).toBeInTheDocument();

    // 点击「vacation」标签 → 只剩带该标签的照片
    fireEvent.click(screen.getByRole('button', { name: 'vacation' }));
    await waitFor(() => {
      expect(screen.queryByAltText('plain.png')).not.toBeInTheDocument();
    });
    expect(screen.getByAltText('tagged.png')).toBeInTheDocument();

    // 再次点击（toggle）取消筛选 → 恢复全部
    fireEvent.click(screen.getByRole('button', { name: 'vacation' }));
    await waitFor(() => {
      expect(screen.getByAltText('plain.png')).toBeInTheDocument();
    });
  });

  it('groups photos by year with separators (需求5：时间分组)', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const old = { ...makeItem('old'), createdAt: '2022-03-01T00:00:00Z' };
    const fresh = { ...makeItem('fresh'), createdAt: '2026-08-10T00:00:00Z' };
    render(<PhotoAlbumOverlay items={[old, fresh]} onClose={vi.fn()} />);

    await screen.findByAltText('old.png');
    // 打开分组下拉并选择「按年」（DropdownSelect 触发按钮可访问名来自 ariaLabel）
    fireEvent.click(screen.getByRole('button', { name: /Group by|album_group_mode/i }));
    fireEvent.click(await screen.findByRole('button', { name: /By year|group_by_year/i }));

    // 两个年份区块标题出现（含对应数量角标）
    await waitFor(() => {
      expect(screen.getByText('2026')).toBeInTheDocument();
      expect(screen.getByText('2022')).toBeInTheDocument();
    });
  });

  it('sorts by createdAt desc by default and toggles to asc (需求5：时间正/倒序)', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const old = { ...makeItem('old'), createdAt: '2022-03-01T00:00:00Z' };
    const fresh = { ...makeItem('fresh'), createdAt: '2026-08-10T00:00:00Z' };
    render(<PhotoAlbumOverlay items={[old, fresh]} onClose={vi.fn()} />);

    await screen.findByAltText('old.png');
    // 默认倒序：最新在前
    const grid = screen.getByTestId('photo-album-grid');
    const cells = within(grid).getAllByRole('button');
    expect(cells[0]).toHaveAttribute('title', 'fresh.png');

    // 点击排序按钮切为正序：最旧在前
    fireEvent.click(
      screen.getByRole('button', { name: /Newest first|Oldest first|sort_asc|sort_desc/i }),
    );
    const gridAsc = screen.getByTestId('photo-album-grid');
    const cellsAsc = within(gridAsc).getAllByRole('button');
    expect(cellsAsc[0]).toHaveAttribute('title', 'old.png');
  });
});
