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
});
