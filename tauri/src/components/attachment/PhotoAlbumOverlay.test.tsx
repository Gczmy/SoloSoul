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

  it('Android 硬件返回：查看器打开时先回网格而非关闭相册，再返回才关闭', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    render(<PhotoAlbumOverlay items={[makeItem('a'), makeItem('b')]} onClose={onClose} />);

    const grid = await screen.findByTestId('photo-album-grid');
    const cells = within(grid).getAllByRole('button');
    fireEvent.click(cells[0]);
    await waitFor(() => {
      expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
    });

    // 第一次返回：查看器层标记被弹出 → 回到网格，相册保持打开
    fireEvent.popState(window);
    await waitFor(() => {
      expect(screen.queryByTestId('photo-viewer-counter')).not.toBeInTheDocument();
    });
    expect(screen.getByTestId('photo-album-grid')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();

    // 第二次返回：相册层标记被弹出 → 关闭相册
    fireEvent.popState(window);
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it('Android 硬件返回：网格态直接关闭相册（回到上一页）', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    render(<PhotoAlbumOverlay items={[makeItem('a')]} onClose={onClose} />);
    await screen.findByAltText('a.png');

    fireEvent.popState(window);
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
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

  it('按对象分组显示 页面→对象 两级结构（页面名 + 缩进对象名）', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    // 两个页面，每个页面两个对象；对象名与页面信息由 collectPhotoItems 从树携带
    const mk = (
      id: string,
      objectId: string,
      objectName: string,
      pageId: string,
      pageName: string,
    ) => ({
      ...makeItem(id),
      objectId,
      objectName,
      pageId,
      pageName,
    });
    const items = [
      mk('a1', 'o1', '身份信息对象1', 'p1', '身份'),
      mk('a2', 'o1', '身份信息对象1', 'p1', '身份'),
      mk('b1', 'o2', '身份信息对象2', 'p1', '身份'),
      mk('c1', 'o3', '护照对象1', 'p2', '旅行'),
    ];
    render(<PhotoAlbumOverlay items={items} onClose={vi.fn()} />);

    await screen.findByAltText('a1.png');
    // 打开分组下拉并选择「按对象」
    fireEvent.click(screen.getByRole('button', { name: /Group by|album_group_mode/i }));
    fireEvent.click(await screen.findByRole('button', { name: /By object|group_by_object/i }));

    // 两级结构：页面名（顶层）+ 对象名（子级）
    await waitFor(() => {
      expect(screen.getByText('身份')).toBeInTheDocument();
      expect(screen.getByText('旅行')).toBeInTheDocument();
      expect(screen.getByText('身份信息对象1')).toBeInTheDocument();
      expect(screen.getByText('身份信息对象2')).toBeInTheDocument();
      expect(screen.getByText('护照对象1')).toBeInTheDocument();
    });

    // 每个页面区块下各有一个缩进的对象子区块网格（照片在其中）
    const grids = screen.getAllByTestId('photo-album-grid');
    expect(grids).toHaveLength(3); // 身份信息对象1 / 身份信息对象2 / 护照对象1
    // 顺序与排序实现相关（页面/对象均按名称字典序），按集合断言避免脆弱
    const gridAlts = grids.map((g) =>
      within(g)
        .getAllByRole('button')
        .map((b) => b.getAttribute('title')),
    );
    const flattened = gridAlts.flat();
    expect(flattened).toEqual(expect.arrayContaining(['a1.png', 'a2.png', 'b1.png', 'c1.png']));
    // 对象级网格划分正确：同一对象的照片在同一网格、跨对象照片不混入
    const groupSizes = gridAlts.map((g) => g.length).sort();
    expect(groupSizes).toEqual([1, 1, 2]);
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
