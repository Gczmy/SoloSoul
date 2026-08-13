import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { PhotoViewerOverlay, swipeNavigation } from './PhotoViewerOverlay';
import { FULL_PREVIEW_MAX_DIM } from '@/lib/photoAlbumPreview';
import type { AttachmentItem } from '@/lib/attachmentUtils';

const mockInvoke = vi.mocked(invoke);

function makeItem(id: string, fileName = `${id}.png`): AttachmentItem {
  return {
    id,
    objectId: 'obj-1',
    fileName,
    mimeType: 'image/png',
    sizeBytes: 100,
    createdAt: '2024-01-01T00:00:00Z',
    vaultPath: `/vault/attachments/obj-1/${fileName}`,
    srcPath: null,
  };
}

describe('swipeNavigation', () => {
  it('maps swipe offsets to page directions', () => {
    expect(swipeNavigation(-100)).toBe(1); // 左滑 → 下一张
    expect(swipeNavigation(100)).toBe(-1); // 右滑 → 上一张
    expect(swipeNavigation(-30)).toBe(0); // 低于阈值不翻页
    expect(swipeNavigation(30)).toBe(0);
  });
});

describe('PhotoViewerOverlay', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('loads full preview, falling back to scaled preview when data url fails', async () => {
    mockInvoke
      .mockRejectedValueOnce(new Error('File too large for preview: 20000000 bytes (max 10485760)'))
      .mockResolvedValueOnce('data:image/jpeg;base64,scaled');
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_file_as_data_url', {
        path: '/vault/attachments/obj-1/a.png',
      });
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_image_preview', {
        path: '/vault/attachments/obj-1/a.png',
        maxDim: FULL_PREVIEW_MAX_DIM,
      });
    });
  });

  it('renders the counter', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <PhotoViewerOverlay
        items={[makeItem('a'), makeItem('b')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
  });

  it('navigates via arrow keys', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <PhotoViewerOverlay
        items={[makeItem('x1'), makeItem('x2')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    fireEvent.keyDown(window, { key: 'ArrowRight' });
    await waitFor(() => {
      expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('2 / 2');
    });

    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
  });

  it('returns to grid via back button', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onBack = vi.fn();
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={onBack}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /common:back_to_album/i }));
    expect(onBack).toHaveBeenCalled();
  });

  it('close button returns to the album instead of closing everything', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onBack = vi.fn();
    const onClose = vi.fn();
    render(
      <PhotoViewerOverlay
        items={[makeItem('a'), makeItem('b')]}
        initialIndex={0}
        onBack={onBack}
        onClose={onClose}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /common:close/i }));
    expect(onBack).toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('renders side nav buttons for multiple photos and navigates on click', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <PhotoViewerOverlay
        items={[makeItem('n1'), makeItem('n2')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const nextBtn = screen.getByRole('button', { name: /common:next/i });
    const prevBtn = screen.getByRole('button', { name: /common:previous/i });
    expect(nextBtn).toBeInTheDocument();
    expect(prevBtn).toBeInTheDocument();

    fireEvent.click(nextBtn);
    expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('2 / 2');
    fireEvent.click(prevBtn);
    expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
  });

  it('hides side nav buttons for a single photo', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.queryByRole('button', { name: /common:next/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /common:previous/i })).not.toBeInTheDocument();
  });

  it('shows error state and open-external fallback when loading fails', async () => {
    // 使用唯一路径，避免命中 photoAlbumPreview 模块级缓存的成功结果
    mockInvoke.mockRejectedValue(new Error('boom'));
    const onOpenExternal = vi.fn();
    render(
      <PhotoViewerOverlay
        items={[makeItem('err-a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
        onOpenExternal={onOpenExternal}
      />,
    );

    expect(await screen.findByText(/common:attachment_preview_failed/i)).toBeInTheDocument();
    fireEvent.click(await screen.findByText(/common:attachment_open_system/i));
    expect(onOpenExternal).toHaveBeenCalled();
  });

  it('calls onClose on Escape', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={onClose}
      />,
    );

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  // ── Android 缩放条加固回归（T009）：真实 WebView 手势层可能吞 click，
  //    主交互路径改 onPointerDown（任何取消前触发）；键盘仍走 click(detail===0)。
  it('zooms via pointerdown (primary mobile path)', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });
    expect(screen.getByText('100%')).toBeInTheDocument();

    fireEvent.pointerDown(screen.getByTitle(/common:attachment_zoom_in/i));
    expect(screen.getByText('120%')).toBeInTheDocument();

    fireEvent.pointerDown(screen.getByTitle(/common:attachment_zoom_out/i));
    expect(screen.getByText('100%')).toBeInTheDocument();

    // pointerdown 后触发的 click(detail>=1) 不应二次缩放
    fireEvent.click(screen.getByTitle(/common:attachment_zoom_in/i), { detail: 1 });
    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  it('does not zoom on right/middle mouse button pointerdown', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });

    fireEvent.pointerDown(screen.getByTitle(/common:attachment_zoom_in/i), { button: 2 });
    expect(screen.getByText('100%')).toBeInTheDocument();

    fireEvent.pointerDown(screen.getByTitle(/common:attachment_zoom_in/i), { button: 1 });
    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  it('zooms via keyboard activation (click with detail 0)', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });

    // 键盘 Enter/Space 激活的 click 事件 detail===0（无 pointerdown），走兜底路径
    fireEvent.click(screen.getByTitle(/common:attachment_zoom_in/i), { detail: 0 });
    expect(screen.getByText('120%')).toBeInTheDocument();
  });

  it('clicking zoom controls never closes the viewer', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    render(
      <PhotoViewerOverlay
        items={[makeItem('a')]}
        initialIndex={0}
        onBack={vi.fn()}
        onClose={onClose}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });

    fireEvent.pointerDown(screen.getByTitle(/common:attachment_zoom_in/i));
    fireEvent.click(screen.getByTitle(/common:attachment_zoom_in/i), { detail: 1 });
    expect(onClose).not.toHaveBeenCalled();
  });
});
