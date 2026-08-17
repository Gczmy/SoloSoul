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

    // 查看器为懒加载（LazyPhotoViewerOverlay 冷启动拉 framer-motion），放宽 waitFor 超时防抖动
    await waitFor(
      () => {
        expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
      },
      { timeout: 8000 },
    );

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
    // 查看器为懒加载（LazyPhotoViewerOverlay 冷启动拉 framer-motion），放宽 waitFor 超时防抖动
    await waitFor(
      () => {
        expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
      },
      { timeout: 8000 },
    );

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

    // 点击「vacation」标签 → 只剩带该标签的照片（chip 现带数量徽标，用正则匹配）
    fireEvent.click(screen.getByRole('button', { name: /vacation/ }));
    await waitFor(() => {
      expect(screen.queryByAltText('plain.png')).not.toBeInTheDocument();
    });
    expect(screen.getByAltText('tagged.png')).toBeInTheDocument();

    // 再次点击（toggle）取消筛选 → 恢复全部
    fireEvent.click(screen.getByRole('button', { name: /vacation/ }));
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

  it('标签筛选区：隐藏滚动条但保留横向滚动（对齐 SearchPopover.filterBar）', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const tagged = makeItem('tagged');
    tagged.tags = ['vacation'];
    render(<PhotoAlbumOverlay items={[tagged]} onClose={vi.fn()} />);

    // 容器具备隐藏滚动条的样式：横向可滚、纵向禁止、滚动条隐藏（Firefox）、
    // chip 不换行（nowrap 由 FilterChipGroup 内联样式提供，容器配合 overflowX）
    const container = document.querySelector('.photo-album-tag-filter') as HTMLElement;
    expect(container).not.toBeNull();
    expect(container.style.overflowX).toBe('auto');
    expect(container.style.overflowY).toBe('hidden');
    expect(container.style.scrollbarWidth).toBe('none');
    // 内联 style 标签存在（webkit 滚动条隐藏规则）
    const styleEls = Array.from(document.querySelectorAll('style'));
    expect(
      styleEls.some((s) => s.textContent?.includes('photo-album-tag-filter::-webkit-scrollbar')),
    ).toBe(true);
  });

  it('顶栏显示全部数量、标签 chip 与工具栏显示当前选项数量', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const tagged = makeItem('tagged');
    tagged.tags = ['vacation'];
    const plain = makeItem('plain');
    render(<PhotoAlbumOverlay items={[tagged, plain]} onClose={vi.fn()} />);

    await screen.findByAltText('tagged.png');

    // ① 顶栏标题右侧 = 全部数量（2），不随筛选变化
    expect(screen.getByTestId('album-total-count').textContent).toBe('2');

    // ② 每个标签 chip 显示该标签的照片数量（vacation=1），「全部」chip 显示总数 2
    const vacationChip = screen.getByRole('button', { name: /vacation/ });
    expect(vacationChip.textContent).toContain('1');

    // ③ 工具栏右侧（排序/分组所在行）显示当前选项数量 2
    expect(screen.getByTestId('album-current-count').textContent).toBe('2');

    // 选择「vacation」后：顶栏仍为全部数量 2，工具栏右侧变为当前选项数量 1
    fireEvent.click(vacationChip);
    await waitFor(() => {
      expect(screen.queryByAltText('plain.png')).not.toBeInTheDocument();
    });
    expect(screen.getByTestId('album-total-count').textContent).toBe('2');
    expect(screen.getByTestId('album-current-count').textContent).toBe('1');
  });

  it('标签筛选区展开/折叠：箭头按钮切换全部标签平铺（高度自适应）与单行滚动', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const tagged = makeItem('tagged');
    tagged.tags = ['vacation'];
    render(<PhotoAlbumOverlay items={[tagged]} onClose={vi.fn()} />);

    // 折叠态：容器约束单行滚动，chip 不换行，按钮为展开箭头（common:expand）
    const container = document.querySelector('.photo-album-tag-filter') as HTMLElement;
    const chipWrap = container.querySelector(':scope > div') as HTMLElement; // FilterChipGroup 容器
    expect(container.style.overflowX).toBe('auto');
    expect(chipWrap.style.flexWrap).toBe('nowrap');
    const expandBtn = screen.getByRole('button', { name: /common:expand/i });
    expect(expandBtn).toBeInTheDocument();

    // 点击展开：容器解除横向约束（overflowX 空）、chip 换行平铺、按钮切换为折叠
    fireEvent.click(expandBtn);
    expect(container.style.overflowX).toBe('');
    expect(chipWrap.style.flexWrap).toBe('wrap');
    expect(screen.getByRole('button', { name: /common:collapse/i })).toBeInTheDocument();

    // 再点折叠：恢复单行滚动约束与展开箭头
    fireEvent.click(screen.getByRole('button', { name: /common:collapse/i }));
    expect(container.style.overflowX).toBe('auto');
    expect(chipWrap.style.flexWrap).toBe('nowrap');
    expect(screen.getByRole('button', { name: /common:expand/i })).toBeInTheDocument();
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
